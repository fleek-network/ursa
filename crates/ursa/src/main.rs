extern crate core;

mod config;
mod ursa;

use std::{path::PathBuf, sync::Arc};

use crate::{
    config::{load_config, UrsaConfig, DEFAULT_CONFIG_PATH_STR},
    ursa::identity::IdentityManager,
};
use async_std::{sync::RwLock, task};
use db::{rocks::RocksDb, rocks_config::RocksDbConfig};
use dotenv::dotenv;
use structopt::StructOpt;
use tracing::{error, info};
use ursa::{cli_error_and_die, wait_until_ctrlc, Cli, Subcommand};
use ursa_index_provider::provider::Provider;
use ursa_metrics::metrics;
use ursa_network::UrsaService;
use ursa_rpc_server::{api::NodeNetworkInterface, server::Server};
use ursa_store::Store;

#[async_std::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();
    load_config(&PathBuf::from(env!("HOME")).join(DEFAULT_CONFIG_PATH_STR));

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
                    mut server_config,
                } = config;
                if opts.rpc_port.is_some() {
                    server_config.port = opts.rpc_port.unwrap();
                }

                let keystore_path = network_config.keystore_path.clone();
                let im = match network_config.identity.clone().as_str() {
                    // ephemeral random identity
                    "random" => IdentityManager::random(),
                    // load or create a new identity
                    _ => {
                        IdentityManager::load_or_new(network_config.identity.clone(), keystore_path)
                    }
                };

                let keypair = im.current();

                let db_path = network_config.database_path.clone();

                info!("Using {:?} as database path", db_path);

                let db = RocksDb::open(db_path, &RocksDbConfig::default())
                    .expect("Opening RocksDB must succeed");
                let db = Arc::new(db);
                let store = Arc::new(Store::new(Arc::clone(&db)));

                let provider_db_name = provider_config.database_path.clone();
                let provider_db = RocksDb::open(provider_db_name, &RocksDbConfig::default())
                    .expect("Opening RocksDB must succeed");
                let index_provider = Provider::new(
                    keypair.clone(),
                    Arc::new(RwLock::new(provider_db)),
                    provider_config.clone(),
                );

                let service = UrsaService::new(
                    keypair,
                    &network_config,
                    Arc::clone(&store),
                    index_provider.clone(),
                );
                let rpc_sender = service.command_sender().clone();

                // Start libp2p service
                let service_task = task::spawn(async {
                    if let Err(err) = service.start().await {
                        error!("[service_task] - {:?}", err);
                    }
                });

                let interface = Arc::new(NodeNetworkInterface {
                    store,
                    network_send: rpc_sender,
                });
                let server = Server::new(interface);

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

                // Start index provider service
                let provider_task = task::spawn(async move {
                    if let Err(err) = index_provider.start(&provider_config).await {
                        error!("[provider_task] - {:?}", err);
                    }
                });

                wait_until_ctrlc();

                // Gracefully shutdown node & rpc
                task::spawn(async {
                    rpc_task.cancel().await;
                    service_task.cancel().await;
                    metrics_task.cancel().await;
                    provider_task.cancel().await;
                });
            }
        }
        Err(e) => {
            cli_error_and_die(&format!("Error parsing config. Error was: {}", e), 1);
        }
    };
}
