use crate::{config::UrsaConfig, ursa::identity::IdentityManager};
use anyhow::{bail, Result};
use db::{rocks::RocksDb, rocks_config::RocksDbConfig};
use dotenv::dotenv;
use resolve_path::PathResolveExt;
use scopeguard::defer;
use std::sync::Arc;
use std::{env, net::SocketAddr};
use structopt::StructOpt;
use tokio::sync::mpsc::channel;
use tokio::task;
use tracing::{error, info};
use ursa::{Cli, Subcommand};
use ursa_application::app::Application;
use ursa_consensus::consensus::Consensus;
use ursa_index_provider::engine::ProviderEngine;
use ursa_network::UrsaService;
use ursa_rpc_service::{api::NodeNetworkInterface, server::Server};
use ursa_store::UrsaStore;
use ursa_telemetry::TelemetryConfig;
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
    // Register the shutdown controller to respect ctrl-c signal.
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
    } = config;

    // Ursa service setup.
    let im = match network_config.identity.as_str() {
        // Ephemeral random identity.
        "random" => IdentityManager::random(),
        // Load or create a new identity.
        _ => IdentityManager::load_or_new(
            network_config.identity.clone(),
            network_config.keystore_path.resolve().to_path_buf(),
        ),
    };

    let keypair = im.current();

    let db_path = network_config.database_path.resolve().to_path_buf();
    info!("Opening blockstore database at {:?}", db_path);

    let db = RocksDb::open(db_path, &RocksDbConfig::default())
        .expect("Opening blockstore RocksDB must succeed");
    let store = Arc::new(UrsaStore::new(Arc::clone(&Arc::new(db))));
    let (event_sender, event_receiver) = channel(4096);
    let service = UrsaService::new(
        keypair.clone(),
        &network_config,
        Arc::clone(&store),
        event_sender,
    )?;

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
        event_receiver,
    );
    let index_provider_router = index_provider_engine.router();

    // Start the application
    let app = Application::new();
    // Get the update and query port
    let (app_update, app_query) = app.get_ports();

    // Server setup.
    let mempool_address = consensus_config.worker[0].transaction.clone();
    let mempool_port = mempool_address
        .iter()
        .find_map(|proto| match proto {
            // Sui and Libp2p are using dif "MAJOR" version of multiaddr so we have to import and use the other one here
            multiaddr::Protocol::Tcp(port) => Some(port),
            _ => None,
        })
        .expect("Expected tcp url for worker mempool");

    let mempool_address_string = format!("http://0.0.0.0:{}", mempool_port);

    // Create engine so we can grab senders.
    // let mut abci_engine = Engine::new(app_address).await;

    // Store the senders from the engine.
    // let tx_abci_queries = abci_engine.get_abci_queries_sender();
    // let tx_certificates = abci_engine.get_certificates_sender();
    // let reconfigure_notify = abci_engine.get_reconfigure_notify();

    // Spawn engine.
    // let engine_shutdown = shutdown_controller.clone();
    // let _abci_engine_task = task::spawn_blocking(|| {
    //     futures::executor::block_on(async move {
    //         if let Err(err) = abci_engine.start(engine_shutdown).await {
    //             error!("[abci_engine_task] - {:?}", err);
    //         }
    //     })
    // });

    let interface = Arc::new(NodeNetworkInterface::new(
        store,
        service.command_sender(),
        index_provider_engine.command_sender(),
        server_config.origin.clone(),
        mempool_address_string.clone(),
    ));

    let server = Server::new(interface);

    // Start libp2p service.
    let shutdown = shutdown_controller.clone();
    let service_task = task::spawn(async move {
        if let Err(err) = service.start().await {
            error!("[service_task] - {:?}", err);
            shutdown.shutdown();
        }
    });

    // todo(oz): spawn task to track storage/ram/cpu metrics
    let metrics = ursa_metrics::routes::init();

    // Start multiplex server service (rpc, http, and metrics).
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

    // Start index provider service.
    let shutdown = shutdown_controller.clone();
    let provider_task = task::spawn(async move {
        if let Err(err) = index_provider_engine.start().await {
            error!("[provider_task] - {:?}", err);
            shutdown.shutdown();
        }
    });

    // Start the consensus service.
    let consensus_handle = task::spawn(async move {
        let mut consensus_service = Consensus::new(
            consensus_config,
            app_query,
            app_update,
            mempool_address_string,
        )
        .unwrap();
        consensus_service.start().await;
    });

    // Wait for the shutdown.
    shutdown_controller.wait_for_shutdown().await;

    // Gracefully shutdown node & rpc.
    rpc_task.abort();
    service_task.abort();
    provider_task.abort();
    consensus_handle.abort();
    Ok(())
}
