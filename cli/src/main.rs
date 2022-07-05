mod ursa;

use std::sync::Arc;

use async_std::task;
use db::rocks::RocksDb;
use dotenv::dotenv;
use libp2p::identity::Keypair;
use network::UrsaService;
use store::Store;
use structopt::StructOpt;
use tracing::{error, info};
use ursa::{cli_error_and_die, Cli};

use crate::ursa::wait_until_ctrlc;

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

                let db = RocksDb::open("test_db").expect("Opening RocksDB must succeed");
                let db = Arc::new(db);

                let store = Arc::new(Store::new(Arc::clone(&db)));
                let service = UrsaService::new(keypair, &config, Arc::clone(&store));

                // Start libp2p service
                let service_task = task::spawn(async {
                    if let Err(err) = service.start().await {
                        error!("[service_task] - {:?}", err);
                    }
                });

                // let config = RpcConfig {
                //     rpc_port: 4069,
                //     rpc_addr: "0.0.0.0".to_string(),
                // };

                // let interface = Arc::new(NodeNetworkInterface { store });
                // let rpc = Rpc::new(&config, interface);

                // Start rpc service
                let rpc_task = task::spawn(async {
                    // if let Err(err) = rpc.start(config).await {
                    //     error!("[rpc_task] - {:?}", err);
                    // }
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
