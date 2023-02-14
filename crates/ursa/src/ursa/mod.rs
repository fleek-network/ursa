use crate::config::{UrsaConfig, DEFAULT_CONFIG_PATH_STR};
use anyhow::Result;
use dirs::home_dir;
use resolve_path::PathResolveExt;
use rpc_commands::RpcCommands;
use std::{path::PathBuf, process};
use structopt::StructOpt;
use tracing::error;

pub mod identity;
mod rpc_commands;

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

#[derive(StructOpt, Debug)]
pub enum Subcommand {
    #[structopt(name = "rpc", about = "run rpc commands from cli")]
    Rpc(RpcCommands),
}

/// CLI options
#[derive(StructOpt, Debug)]
pub struct CliOpts {
    #[structopt(short, long, help = "A toml file containing relevant configurations")]
    pub config: Option<PathBuf>,
    #[structopt(short, long, help = "Allow rpc to be active or not (default = true)")]
    pub rpc: bool,
    #[structopt(short = "p", long, help = "Port used for JSON-RPC communication")]
    pub rpc_port: Option<u16>,
    #[structopt(
        short,
        long,
        help = "Set logging level: info (default), error, warn, debug, trace"
    )]
    pub log: Option<String>,
}

impl CliOpts {
    pub fn to_config(&self) -> Result<UrsaConfig> {
        let mut config = UrsaConfig::load_or_default(
            &self
                .config
                .as_ref()
                .map(|p| p.resolve().to_path_buf())
                .unwrap_or_else(|| home_dir().unwrap_or_default().join(DEFAULT_CONFIG_PATH_STR)),
        )?;

        if let Some(rpc_port) = self.rpc_port {
            config.server_config.port = rpc_port;
        }

        Ok(config)
    }
}

/// Print an error message and exit the program with an error code
/// Used for handling high level errors such as invalid params
pub(super) fn cli_error_and_die(msg: &str, code: i32) {
    error!("Error: {}", msg);
    process::exit(code);
}
