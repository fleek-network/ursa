mod rpc_commands;

use crate::config::{UrsaConfig, DEFAULT_CONFIG_PATH_STR};
use rpc_commands::RpcCommands;
use std::cell::RefCell;
use std::fs::File;
use std::io::{prelude::*, Result};
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{process, thread};
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
}

impl CliOpts {
    pub fn to_config(&self) -> Result<UrsaConfig> {
        let mut path = PathBuf::from(env!("HOME")).join(DEFAULT_CONFIG_PATH_STR);
        if let Some(config_file) = &self.config {
            info!(
                "Reading configuration from user provided config file {}",
                config_file
            );
            path = PathBuf::from(&config_file);
        }

        // Read from config file
        let toml = read_file_to_string(&path).unwrap();
        // Parse and return the configuration file
        let toml_str: UrsaConfig = toml::from_str(&toml).unwrap();

        Ok(toml_str)
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
    process::exit(code);
}
