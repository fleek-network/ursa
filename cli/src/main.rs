mod ursa;
// mod node;

use ursa::{cli_error_and_die, Cli};
use structopt::StructOpt;
// use node::service::FnetService;

#[async_std::main]
async fn main() {
    // Capture Cli inputs
    let Cli { opts, cmd } = Cli::from_args();

    // TODO: store the config in memory for node startup
    match opts.to_config() {
        Ok(cfg) => match cmd {
            Some(command) => todo!(),
            None => {
                println!("Starting up with config {:?}", cfg);
                // let service = service::FnetService::new(cfg, store);
                // service.start().await;
                
            }
        },
        Err(e) => {
            cli_error_and_die(&format!("Error parsing config. Error was: {}", e), 1);
        }
    };
}
