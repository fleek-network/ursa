use crate::{config::UrsaConfig, ursa::identity::IdentityManager};
use anyhow::Result;
use db::{rocks::RocksDb, rocks_config::RocksDbConfig};
use dotenv::dotenv;
use libp2p::multiaddr::Protocol;
use resolve_path::PathResolveExt;
use std::env;
use std::sync::Arc;
use structopt::StructOpt;
use tokio::{sync::mpsc::channel, task};
use tracing::{error, info};
use ursa::{cli_error_and_die, wait_until_ctrlc, Cli, Subcommand};
use ursa_consensus::{
    execution::Execution,
    service::{ConsensusService, ServiceArgs},
};
use ursa_index_provider::engine::ProviderEngine;
use ursa_network::{ursa_agent, UrsaService};
use ursa_rpc_service::{api::NodeNetworkInterface, server::Server};
use ursa_store::UrsaStore;
use ursa_telemetry::TelemetryConfig;
use ursa_tracker::TrackerRegistration;

pub mod config;
mod ursa;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let Cli { opts, cmd } = Cli::from_args();
    let log_level = env::var("RUST_LOG").unwrap_or_else(|_| "INFO".to_string());

    TelemetryConfig::new("ursa-cli")
        .with_pretty_log()
        .with_log_level(opts.log.as_ref().unwrap_or(&log_level))
        .init()?;

    match opts.to_config() {
        Ok(config) => {
            if let Some(command) = cmd {
                match command {
                    Subcommand::Rpc(cmd) => {
                        cmd.run().await;
                    }
                }
            } else {
                let UrsaConfig {
                    network_config,
                    provider_config,
                    server_config,
                    consensus_config,
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

                let consensus_args = ServiceArgs::load(consensus_config).unwrap();

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
                let service =
                    UrsaService::new(keypair.clone(), &network_config, Arc::clone(&store))?;

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
                let service_task = task::spawn(async {
                    if let Err(err) = service.start().await {
                        error!("[service_task] - {:?}", err);
                    }
                });

                // todo(oz): spawn task to track storage/ram/cpu metrics
                let metrics = ursa_metrics::routes::init();

                // Start multiplex server service (rpc, http, and metrics)
                let rpc_task = task::spawn(async move {
                    if let Err(err) = server
                        .start(&server_config, index_provider_router, Some(metrics))
                        .await
                    {
                        error!("[rpc_task] - {:?}", err);
                    }
                });

                // Start index provider service
                let provider_task = task::spawn(async move {
                    if let Err(err) = index_provider_engine.start().await {
                        error!("[provider_task] - {:?}", err);
                    }
                });

                // Start the consensus service.
                let consensus_service = ConsensusService::new(consensus_args);
                let (tx_transactions, _rx_transactions) = channel(100);
                let execution = Execution::new(0, tx_transactions);
                consensus_service.start(execution).await;

                // register with ursa node tracker
                if !network_config.tracker.is_empty() {
                    match ursa_tracker::register_with_tracker(network_config.tracker, registration)
                        .await
                    {
                        Ok(res) => info!("Registered with tracker: {res:?}"),
                        Err(err) => error!("Failed to register with tracker: {err:?}"),
                    }
                }

                wait_until_ctrlc();

                // Gracefully shutdown node & rpc
                rpc_task.abort();
                service_task.abort();
                provider_task.abort();
                consensus_service.shutdown().await;
            }
        }
        Err(e) => {
            cli_error_and_die(&format!("Config error: {e}"), 1);
        }
    };

    TelemetryConfig::teardown();
    Ok(())
}
