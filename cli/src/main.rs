mod ursa;

use std::sync::Arc;

use async_std::task;
use db::rocks::RocksDb;
use dotenv::dotenv;
use libp2p::identity::Keypair;
use network::UrsaService;
use rpc_server::{api::NodeNetworkInterface, config::RpcConfig, server::Rpc};
use store::Store;
use structopt::StructOpt;
use tracing::{error, info};
use ursa::{cli_error_and_die, wait_until_ctrlc, Cli};

#[async_std::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::fmt::init();

    // Capture Cli inputs
    let Cli { opts, cmd } = Cli::from_args();

    match opts.to_config() {
        Ok(config) => {
            if let Some(_) = cmd {
                todo!()
            } else {
                info!("Starting up with config {:?}", config);

                // todo(botch): check local for a stored keypair
                let keypair = Keypair::generate_ed25519();
                let db_path = opts
                    .database_path
                    .unwrap_or(network::DEFAULT_DATABASE_PATH.to_string());

                let db = RocksDb::open(db_path).expect("Opening RocksDB must succeed");
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
                let RpcConfig { rpc_addr, rpc_port } = RpcConfig::default();
                let port = opts.port.unwrap_or(rpc_port);
                let rpc_config = RpcConfig::new(port, rpc_addr);

                let interface = Arc::new(NodeNetworkInterface {
                    store,
                    network_send: rpc_sender,
                });
                let rpc = Rpc::new(&rpc_config, interface);

                // Start rpc service
                let rpc_task = task::spawn(async move {
                    if let Err(err) = rpc.start(rpc_config).await {
                        error!("[rpc_task] - {:?}", err);
                    }
                });

                wait_until_ctrlc();

                // Gracefully shutdown node & rpc
                task::spawn(async {
                    rpc_task.cancel().await;
                    service_task.cancel().await;
                });
            }
        }
        Err(e) => {
            cli_error_and_die(&format!("Error parsing config. Error was: {}", e), 1);
        }
    };
}
