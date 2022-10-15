mod rpc_commands;

use network::UrsaConfig;
use rpc_commands::RpcCommands;
use std::{
    cell::RefCell,
    fs::{File},
    io::{prelude::*, Result},
    path::{Path,PathBuf},
    sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}},
    time::Duration,
    process, thread,
};
use libp2p::Multiaddr;
use structopt::StructOpt;
use tracing::{error, info, warn};

pub mod identity;

/// CLI structure generated when interacting with URSA binary
#[derive(StructOpt)]
#[structopt(
    name = option_env!("CARGO_PKG_NAME").unwrap_or("ursa"),
    version = option_env!("URSA_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")),
    about = option_env!("CARGO_PKG_DESCRIPTION").unwrap_or("The Great Bear"),
    author = option_env!("CARGO_PKG_AUTHORS").unwrap_or("Fleek")
)]
pub struct Cli {
    #[structopt(flatten)]
    pub opts: CliOpts,

    #[structopt(subcommand)]
    pub cmd: Option<Subcommand>,
}

#[derive(StructOpt)]
pub enum Subcommand {
    #[structopt(name = "rpc", about = "run rpc commands from cli")]
    Rpc(RpcCommands),
}

/// CLI options
#[derive(StructOpt, Debug)]
pub struct CliOpts {
    #[structopt(short, long, help = "A toml file containing relevant configurations")]
    pub config: Option<String>,
    #[structopt(short, long, help = "Allow rpc to be active or not (default = true)")]
    pub rpc: bool,
    #[structopt(short, long, help = "Port used for JSON-RPC communication")]
    pub rpc_port: Option<u16>,
    #[structopt(short, long, help = "Database path where store data")]
    pub database_path: Option<String>,
    #[structopt(short, long, help = "Path to the keystore directory. Defaults to ~/.config/ursa/keystore")]
    pub keystore_path: Option<String>,
    #[structopt(short, long, help = "Identity name. If not provided, a default identity will be created and reused automatically")]
    pub identity: Option<String>,
    #[structopt(short, long, help = "Swarm address")]
    pub swarm_addr: Option<Multiaddr>,
}

impl CliOpts {
    pub fn to_config(&self) -> Result<UrsaConfig> {
        let mut cfg = UrsaConfig::default();
        if let Some(config_file) = &self.config {
            info!(
                "Reading configuration from user provided config file {}",
                config_file
            );
            // Read from config file
            let toml = read_file_to_string(&PathBuf::from(&config_file)).unwrap();
            // Parse and return the configuration file
            let toml_str: UrsaConfig = toml::from_str(&toml).unwrap();
            cfg = toml_str.merge(cfg);
        }

        if let Some(identity) = &self.identity {
            cfg.identity = identity.to_string();
        }

        if let Some(swarm_addr) = &self.swarm_addr {
            cfg.swarm_addr = swarm_addr.clone();
        }

        Ok(cfg)
    }
}

/// Read file as a `String`.
pub fn read_file_to_string(path: &Path) -> Result<String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) => {
            error!("Problem opening the file: {:?}", error);
            process::exit(1);
        }
    };
    let mut string = String::new();
    file.read_to_string(&mut string)?;
    Ok(string)
}

pub fn wait_until_ctrlc() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(500));
    }
}

/// Blocks current thread until ctrl-c is received
pub async fn _block_until_sigint() {
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
