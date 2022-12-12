use crate::config::DEFAULT_URSA_GATEWAY_CONFIG_PATH;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)] // Read from `Cargo.toml`
#[command(propagate_version = true)]
pub struct Cli {
    #[arg(long, default_value_t = format!("{}/{}", env!("HOME"), DEFAULT_URSA_GATEWAY_CONFIG_PATH))]
    pub config: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run gateway daemon
    Daemon(DaemonCmdOpts),
}

/// Override config
#[derive(Args)]
pub struct DaemonCmdOpts {
    /// Server port
    #[arg(long)]
    pub port: Option<u16>,
    /// Server address
    #[arg(long)]
    pub addr: Option<String>,
    /// Cert path
    #[arg(long)]
    pub cert_path: Option<PathBuf>,
    /// Key path
    #[arg(long)]
    pub key_path: Option<PathBuf>,
}
