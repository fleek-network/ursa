use crate::config::DEFAULT_URSA_GATEWAY_CONFIG_PATH;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)] // Read from `Cargo.toml`
#[command(propagate_version = true)]
pub struct Cli {
    /// config path
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
    /// server port
    #[arg(long)]
    pub port: Option<u16>,
    /// server address
    #[arg(long)]
    pub addr: Option<String>,
    /// tls cert path
    #[arg(long)]
    pub tls_cert_path: Option<PathBuf>,
    /// tls key path
    #[arg(long)]
    pub tls_key_path: Option<PathBuf>,
}
