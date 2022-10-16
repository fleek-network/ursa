extern crate core;

mod ursa;

use crate::ursa::identity::IdentityManager;
use async_std::task;
use db::{rocks::RocksDb, rocks_config::RocksDbConfig};
use dotenv::dotenv;
use std::sync::Arc;
use structopt::StructOpt;
use tracing::{error, info};
use ursa::{cli_error_and_die, wait_until_ctrlc, Cli, Subcommand};
use ursa_metrics::{config::MetricsServiceConfig, metrics};
use ursa_network::UrsaService;
use ursa_rpc_server::{api::NodeNetworkInterface, config::ServerConfig, server::Server};
use ursa_store::Store;

#[async_std::main]
async fn main() {
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
                info!("UrsaConfig: {:?}", config);

                let keystore_path = config.keystore_path.clone();
                let im = match config.identity.clone().as_str() {
                    // ephemeral random identity
                    "random" => IdentityManager::random(),
                    // load or create a new identity
                    _ => IdentityManager::load_or_new(config.identity.clone(), keystore_path),
                };

                let keypair = im.current();

                let db_path = if let Some(path) = opts.database_path {
                    path
                } else {
                    config
                        .database_path
                        .as_ref()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string()
                };

                info!("Using {} as database path", db_path);

                let db = RocksDb::open(db_path, &RocksDbConfig::default())
                    .expect("Opening RocksDB must succeed");
                let db = Arc::new(db);
                let store = Arc::new(Store::new(Arc::clone(&db)));
                let service = UrsaService::new(keypair, &config, Arc::clone(&store));
                let rpc_sender = service.command_sender().clone();

                // Start libp2p service
                let service_task = task::spawn(async {
                    if let Err(err) = service.start().await {
                        error!("[service_task] - {:?}", err);
                    }
                });

                let ServerConfig { addr, port } = ServerConfig::default();
                let port = opts.rpc_port.unwrap_or(port);
                let server_config = ServerConfig::new(port, addr);

                let interface = Arc::new(NodeNetworkInterface {
                    store,
                    network_send: rpc_sender,
                });
                let server = Server::new(&server_config, interface);

                let metrics_config = MetricsServiceConfig::default();

                // Start multiplex server service(rpc and http)
                let rpc_task = task::spawn(async move {
                    if let Err(err) = server.start(server_config).await {
                        error!("[server] - {:?}", err);
                    }
                });

                // Start metrics service
                let metrics_task = task::spawn(async move {
                    if let Err(err) = metrics::start(&metrics_config).await {
                        error!("[metrics_task] - {:?}", err);
                    }
                });

                wait_until_ctrlc();

                // Gracefully shutdown node & rpc
                task::spawn(async {
                    rpc_task.cancel().await;
                    service_task.cancel().await;
                    metrics_task.cancel().await;
                });
            }
        }
        Err(e) => {
            cli_error_and_die(&format!("Error parsing config. Error was: {}", e), 1);
        }
    };
}
