mod config;

use tracing::{info, warn, error};
use std::cell::RefCell;
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::path::Path;
use structopt::StructOpt;
use toml;
use std::fs::File;
use std::io::{prelude::*, Result};

pub use self::config::CliConfig as Config;

/// CLI structure generated when interacting with URSA binary
#[derive(StructOpt)]
#[structopt(
    name = "ursa",// env!("CARGO_PKG_NAME"),
    version = "0.1", // option_env!("URSA_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")),
    about = "The Great Bear",// env!("CARGO_PKG_DESCRIPTION"),
    // author = "",// env!("CARGO_PKG_AUTHORS")
)]
pub struct Cli {
    #[structopt(flatten)]
    pub opts: CliOpts,

    #[structopt(subcommand)]
    pub cmd: Option<Subcommand>,
}
#[derive(StructOpt)]
pub enum Subcommand {
    // TODO:  be implemented when we add subcommands to this cli
    // e.g.
    // #[structopt(
    //     name = "fetch-params",
    //     about = "Download parameters for generating and verifying proofs for given size"
    // )]
    // Fetch(FetchCommands),
}

/// CLI options
#[derive(StructOpt, Debug)]
pub struct CliOpts {
    #[structopt(short, long, help = "A toml file containing relevant configurations")]
    pub config: Option<String>,
    #[structopt(short, long, help = "Allow rpc to be active or not (default = true)")]
    pub rpc: bool,
    #[structopt(short, long, help = "Port used for JSON-RPC communication", requires("rpc"))]
    pub port: Option<String>,
}

impl CliOpts {
    pub fn to_config(&self) -> Result<Config> {
        let cfg: Config = match &self.config {
            Some(config_file) => {
                info!("Reading configuration from user provided config file {}", config_file);
                // Read from config file
                let toml = read_file_to_string(&PathBuf::from(&config_file)).unwrap();
                // Parse and return the configuration file
                // read_toml(&toml)?
                let toml_str = toml::from_str(&toml).unwrap();
                toml_str 
            }
            None => Config::default(),
        };
        Ok(cfg)
    }
}

/// Read file as a `String`.
pub fn read_file_to_string(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut string = String::new();
    file.read_to_string(&mut string)?;
    Ok(string)
}

/// Blocks current thread until ctrl-c is received
pub async fn block_until_sigint() {
    let (ctrlc_send, ctrlc_oneshot) = futures::channel::oneshot::channel();
    let ctrlc_send_c = RefCell::new(Some(ctrlc_send));

    let running = Arc::new(AtomicUsize::new(0));
    ctrlc::set_handler(move || {
        let prev = running.fetch_add(1, Ordering::SeqCst);
        if prev == 0 {
            warn!("Got interrupt, shutting down...");
            // Send sig int in channel to blocking task
            if let Some(ctrlc_send) = ctrlc_send_c.try_borrow_mut().unwrap().take() {
                ctrlc_send.send(()).expect("Error sending ctrl-c message");
            }
        } else {
            process::exit(0);
        }
    })
    .expect("Error setting Ctrl-C handler");

    ctrlc_oneshot.await.unwrap();
}

/// Print an error message and exit the program with an error code
/// Used for handling high level errors such as invalid params
pub(super) fn cli_error_and_die(msg: &str, code: i32) {
    error!("Error: {}", msg);
    std::process::exit(code);
}
