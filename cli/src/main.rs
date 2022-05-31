mod ursa;

use dotenv::dotenv;
use network::service::UrsaService;
use std::env;
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
        Ok(cfg) => match cmd {
            Some(command) => todo!(),
            None => {
                info!("Starting up with config {:?}", cfg);
                let service = UrsaService::new(cfg, store);
                service.start().await;
            }
        },
        Err(e) => {
            cli_error_and_die(&format!("Error parsing config. Error was: {}", e), 1);
        }
    };
}
