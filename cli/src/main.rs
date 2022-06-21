mod ursa;

use anyhow::{anyhow, Error, Ok, Result};
use async_std::task;
use db::rocks::RocksDb;
use dotenv::dotenv;
use futures::channel::oneshot;
use network::service::UrsaService;
use std::{cell::RefCell, sync::Arc};
use store::Store;
use structopt::StructOpt;
use tracing::{error, info};
use ursa::{cli_error_and_die, Cli};

fn wait_until_ctrlc() -> Result<(), Error> {
    let (ctrlc_send, ctrlc_oneshot) = oneshot::channel();
    let ctrlc_send_c = RefCell::new(Some(ctrlc_send));

    ctrlc::set_handler(move || {
        if let Some(ctrlc_send) = ctrlc_send_c.try_borrow_mut().unwrap().take() {
            if let Err(e) = ctrlc_send.send(()) {
                error!("Error sending ctrl-c message");
            }
        }
    })
    .map_err(|e| anyhow!("Could not set ctrlc handler: {:?}", e))
}

#[async_std::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::fmt::init();

    // Capture Cli inputs
    let Cli { opts, cmd } = Cli::from_args();

    match opts.to_config() {
        Ok(config) => {
            if let Some(command) = cmd {
                todo!()
            } else {
                info!("Starting up with config {:?}", config);

                let db = RocksDb::open("test_db").expect("Opening RocksDB must succeed");
                let db = Arc::new(db);

                let store = Arc::new(Store::new(Arc::clone(&db)));
                let service = UrsaService::new(&config, Arc::clone(&store));

                // Start libp2p service
                let service_task = task::spawn(async {
                    if let Err(err) = service.start().await {
                        error!("[service_task] - {:?}", err);
                    }
                });

                // Start rpc service
                let rpc_task = task::spawn(async {
                    // if let Err(err) = service.start().await {
                    //     error!("[rpc_task] - {:?}", err);
                    // }
                });

                // Gracefully shutdown
                rpc_task.cancel();
                service_task.cancel();
            }
        }
        Err(e) => {
            cli_error_and_die(&format!("Error parsing config. Error was: {}", e), 1);
        }
    };
}
