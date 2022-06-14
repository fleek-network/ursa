mod ursa;

use db::rocks::RocksDb;
use dotenv::dotenv;
use network::service::UrsaService;
use std::{env, sync::Arc};
use store::Store;
use structopt::StructOpt;
use tracing::{error, info, warn};
use tracing_subscriber;
use ursa::{cli_error_and_die, Cli};

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

                service.start().await;
            }
        }
        Err(e) => {
            cli_error_and_die(&format!("Error parsing config. Error was: {}", e), 1);
        }
    };
}
