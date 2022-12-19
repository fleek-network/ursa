use crate::{
    config::{UrsaConfig, DEFAULT_CONFIG_PATH_STR},
    ursa::identity::IdentityManager,
};
use anyhow::{anyhow, Error, Result};
use db::{rocks::RocksDb, rocks_config::RocksDbConfig};
use dotenv::dotenv;
use libp2p::multiaddr::Protocol;
use resolve_path::PathResolveExt;
use std::sync::Arc;
use structopt::StructOpt;
use tokio::task;
use tracing::{error, info};
use ursa::{cli_error_and_die, wait_until_ctrlc, Cli, Subcommand};
use ursa_metrics::server;
use ursa_network::UrsaService;
use ursa_rpc_server::{api::NodeNetworkInterface, server::Server};
use ursa_tracker::TrackerRegistration;
use ursa_index_provider::{engine::ProviderEngine, provider::Provider};
use ursa_store::UrsaStore;

pub mod config;
mod ursa;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    // Capture Cli inputs
    let Cli { opts, cmd } = Cli::from_args();

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
                    metrics_config,
                    server_config,
                } = config;

                // ursa service setup
                let keystore_path = network_config.keystore_path.clone();
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

                let registration = TrackerRegistration {
                    id: keypair.clone().public().to_peer_id(),
                    // TODO: calculate or get from config the supplied storage in bytes
                    storage: 0,
                    agent: format!("ursa/{}", env!("CARGO_PKG_VERSION")),
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
                    rpc_port: Some(server_config.port),
                    metrics_port: Some(metrics_config.port),
                };

                let db_path = network_config.database_path.resolve().to_path_buf();
                info!("Opening blockstore database at {:?}", db_path);

                let db = RocksDb::open(db_path, &RocksDbConfig::default())
                    .expect("Opening blockstore RocksDB must succeed");
                let store = Arc::new(UrsaStore::new(Arc::clone(&Arc::new(db))));
                let service =
                    UrsaService::new(keypair.clone(), &network_config, Arc::clone(&store))?;

                let provider_db = RocksDb::open(
                    &provider_config.database_path.resolve(),
                    &RocksDbConfig::default(),
                )
                .expect("Opening provider RocksDB must succeed");

                let index_store = Arc::new(UrsaStore::new(Arc::clone(&Arc::new(provider_db))));
                let index_provider_engine = ProviderEngine::new(
                    keypair.clone(),
                    Arc::clone(&store),
                    index_store,
                    provider_config.clone(),
                    service.command_sender(),
                );

                // Start metrics service
                let metrics_task = task::spawn(async move {
                    if let Err(err) = server::start(&metrics_config).await {
                        error!("[metrics_task] - {:?}", err);
                    }
                });

                let service = UrsaService::new(keypair, &network_config, Arc::clone(&store))?;
                let rpc_sender = service.command_sender();

                // server setup
                let interface = Arc::new(NodeNetworkInterface {
                    store,
                    network_send: service.command_sender(),
                    provider_send: index_provider_engine.command_sender(),
                });
                let server = Server::new(interface);

                // Start libp2p service
                let service_task = task::spawn(async {
                    if let Err(err) = service.start().await {
                        error!("[service_task] - {:?}", err);
                    }
                });

                // Start multiplex server service(rpc and http)
                let rpc_task = task::spawn(async move {
                    if let Err(err) = server.start(&server_config).await {
                        error!("[rpc_task] - {:?}", err);
                    }
                });

                // Start index provider service
                let provider_task = task::spawn(async move {
                    if let Err(err) = index_provider_engine.start().await {
                        error!("[provider_task] - {:?}", err);
                    }
                });

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
                metrics_task.abort();
                provider_task.abort();
            }
        }
        Err(e) => {
            cli_error_and_die(&format!("Config error: {}", e), 1);
        }
    };
    Ok(())
}
