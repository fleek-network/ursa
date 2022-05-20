mod ursa;

use std::env;
use dotenv::dotenv;
// use node::service::FnetService;
use tracing::{info, warn, error};
use tracing_subscriber;
use ursa::{cli_error_and_die, Cli};
use structopt::StructOpt;
use node::service::FnetService;

#[async_std::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();
    // let collector = tracing_subscriber::fmt()
    // // filter spans/events with level TRACE or higher.
    // .with_max_level(Level::TRACE) //env!("LOG_LEVEL")
    // // build but do not install the subscriber.
    // .finish();
    // Capture Cli inputs
    let Cli { opts, cmd } = Cli::from_args();

    match opts.to_config() {
        Ok(cfg) => match cmd {
            Some(command) => todo!(),
            None => {
                info!("Starting up with config {:?}", cfg);
                let service = FnetService::new(cfg, store);
                service.start().await;
                
            }
        },
        Err(e) => {
            cli_error_and_die(&format!("Error parsing config. Error was: {}", e), 1);
        }
    };
}
