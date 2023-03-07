use crate::{config::UrsaConfig, ursa::identity::IdentityManager};
use anyhow::{bail, Result};
use db::{rocks::RocksDb, rocks_config::RocksDbConfig};
use dotenv::dotenv;
use libp2p::multiaddr::Protocol;
use resolve_path::PathResolveExt;
use scopeguard::defer;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use structopt::StructOpt;
use tokio::{sync::mpsc::channel, task};
use tracing::{error, info};
use ursa::{Cli, Subcommand};
use ursa_application::application_start;
use ursa_consensus::{
    execution::Execution,
    service::{ConsensusService, ServiceArgs},
    AbciApi, Engine,
};
use ursa_index_provider::engine::ProviderEngine;
use ursa_network::{ursa_agent, UrsaService};
use ursa_rpc_service::{api::NodeNetworkInterface, server::Server};
use ursa_store::UrsaStore;
use ursa_telemetry::TelemetryConfig;
use ursa_tracker::TrackerRegistration;
use ursa_utils::shutdown::ShutdownController;

pub mod config;
mod ursa;

#[tokio::main]
async fn main() {
    dotenv().ok();

    if let Err(err) = run().await {
        error!("Error running ursa: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let Cli { cmd, opts } = Cli::from_args();
    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "INFO".to_string());

    TelemetryConfig::new("ursa-cli")
        .with_pretty_log()
        .with_log_level(opts.log.as_ref().unwrap_or(&log_level))
        .init()?;

    // Make sure we run teardown no matter how we exit the run function.
    defer! { TelemetryConfig::teardown(); }

    // Construct a single instance of shutdown controller for the entire application.
    // This instance should be cloned and passed down to whoever that needs it and not
    // reconstructed.
    let shutdown_controller = ShutdownController::default();
    // register the shutdown controller to respect ctrl-c signal.
    shutdown_controller.install_ctrl_c_handler();

    let config = match opts.to_config() {
        Ok(config) => config,
        Err(error) => {
            bail!("Config error: {error}");
        }
    };

    if let Some(Subcommand::Rpc(cmd)) = cmd {
        // TODO(qti3e) cmd.run should return a Result.
        cmd.run().await;
        return Ok(());
    }

    let UrsaConfig {
        network_config,
        provider_config,
        server_config,
        consensus_config,
        application_config,
    } = config;

    // ursa service setup
    let im = match network_config.identity.as_str() {
        // ephemeral random identity
        "random" => IdentityManager::random(),
        // load or create a new identity
        _ => IdentityManager::load_or_new(
            network_config.identity.clone(),
            network_config.keystore_path.resolve().to_path_buf(),
        ),
    };

    let keypair = im.current();

    let consensus_args = ServiceArgs::load(consensus_config.clone()).unwrap();

    let registration = TrackerRegistration {
        id: keypair.clone().public().to_peer_id(),
        agent: ursa_agent(),
        addr: None, // if we have a dns address, we can set it here
        p2p_port: network_config
            .swarm_addrs
            .first()
            .expect("no tcp swarm address")
            .iter()
            .find_map(|proto| match proto {
                Protocol::Tcp(port) => Some(port),
                Protocol::Udp(port) => Some(port),
                _ => None,
            }),
        http_port: Some(server_config.port),
        telemetry: Some(true),
    };

    let db_path = network_config.database_path.resolve().to_path_buf();
    info!("Opening blockstore database at {:?}", db_path);

    let db = RocksDb::open(db_path, &RocksDbConfig::default())
        .expect("Opening blockstore RocksDB must succeed");
    let store = Arc::new(UrsaStore::new(Arc::clone(&Arc::new(db))));
    let service = UrsaService::new(keypair.clone(), &network_config, Arc::clone(&store))?;

    let provider_db = RocksDb::open(
        provider_config.database_path.resolve(),
        &RocksDbConfig::default(),
    )
    .expect("Opening provider RocksDB must succeed");

    let index_store = Arc::new(UrsaStore::new(Arc::clone(&Arc::new(provider_db))));
    let index_provider_engine = ProviderEngine::new(
        keypair,
        Arc::clone(&store),
        index_store,
        provider_config,
        service.command_sender(),
        server_config.addresses.clone(),
    );
    let index_provider_router = index_provider_engine.router();

    // server setup
    let interface = Arc::new(NodeNetworkInterface::new(
        store,
        service.command_sender(),
        index_provider_engine.command_sender(),
        server_config.origin.clone(),
    ));
    let server = Server::new(interface);

    // Start libp2p service
    let shutdown = shutdown_controller.clone();
    let service_task = task::spawn(async move {
        if let Err(err) = service.start().await {
            error!("[service_task] - {:?}", err);
            shutdown.shutdown();
        }
    });

    // todo(oz): spawn task to track storage/ram/cpu metrics
    let metrics = ursa_metrics::routes::init();

    // Start multiplex server service (rpc, http, and metrics)
    let shutdown = shutdown_controller.clone();
    let rpc_task = task::spawn(async move {
        if let Err(err) = server
            .start(&server_config, index_provider_router, Some(metrics))
            .await
        {
            error!("[rpc_task] - {:?}", err);
            shutdown.shutdown();
        }
    });

    // Start index provider service
    let shutdown = shutdown_controller.clone();
    let provider_task = task::spawn(async move {
        if let Err(err) = index_provider_engine.start().await {
            error!("[provider_task] - {:?}", err);
            shutdown.shutdown();
        }
    });

    //Store this to pass to consensus engine
    let app_api = application_config.domain.clone();

    // Start the application server
    let application_task = task::spawn(async move {
        if let Err(err) = application_start(application_config).await {
            error!("[application_task] - {:?}", err)
        }
    });

    //TODO(dalton): This is temporary this shim should be merged with the ursa-rpc-service
    //Start the ABCI shim and engine
    let (tx_abci_queries, rx_abci_queries) = channel(1000);
    let mempool_address = consensus_config.worker[0].transaction.clone();

    let abci_task = task::spawn(async move {
        let api = AbciApi::new(mempool_address, tx_abci_queries).await;
        let address = consensus_config.rpc_domain.parse::<SocketAddr>().unwrap();
        warp::serve(api.routes()).run(address).await;
    });

    // Start the consensus service.
    let consensus_service = ConsensusService::new(consensus_args);
    let (tx_transactions, rx_transactions) = channel(1000);
    let execution = Execution::new(0, tx_transactions);
    consensus_service.start(execution).await;

    let consensus_engine_task = task::spawn(async move {
        let mut app_address = app_api.parse::<SocketAddr>().unwrap();
        app_address.set_ip("0.0.0.0".parse().unwrap());

        let mut engine = Engine::new(app_address, rx_abci_queries);

        if let Err(err) = engine.run(rx_transactions).await {
            error!("[consensus_engine_task] - {:?}", err)
        }
    });

    // register with ursa node tracker
    if !network_config.tracker.is_empty() {
        match ursa_tracker::register_with_tracker(network_config.tracker, registration).await {
            Ok(res) => info!("Registered with tracker: {res:?}"),
            // if tracker fails, keep the process open.
            Err(err) => error!("Failed to register with tracker: {err:?}"),
        }
    }

    // wait for the shutdown.
    shutdown_controller.wait_for_shutdown().await;

    // Gracefully shutdown node & rpc
    rpc_task.abort();
    service_task.abort();
    provider_task.abort();
    consensus_engine_task.abort();
    application_task.abort();
    abci_task.abort();
    consensus_service.shutdown().await;

    Ok(())
}
