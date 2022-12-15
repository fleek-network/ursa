use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use tracing::Level;

use crate::config::DEFAULT_URSA_GATEWAY_CONFIG_PATH;

#[derive(Parser)]
#[command(author, version, about, long_about = None)] // Read from `Cargo.toml`
#[command(propagate_version = true)]
pub struct Cli {
    /// log level
    #[arg(long)]
    pub log: Option<Level>,

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
    pub server_port: Option<u16>,
    /// server address
    #[arg(long)]
    pub server_addr: Option<String>,
    /// server tls cert path
    #[arg(long)]
    pub server_tls_cert_path: Option<PathBuf>,
    /// server tls key path
    #[arg(long)]
    pub server_tls_key_path: Option<PathBuf>,
    /// admin port
    #[arg(long)]
    pub admin_port: Option<u16>,
    /// admin address
    #[arg(long)]
    pub admin_addr: Option<String>,
    /// admin tls cert path
    #[arg(long)]
    pub admin_tls_cert_path: Option<PathBuf>,
    /// admin tls key path
    #[arg(long)]
    pub admin_tls_key_path: Option<PathBuf>,
    /// indexer cid url
    #[arg(long)]
    pub indexer_cid_url: Option<String>,
    /*
     * /// indexer mh url
     * #[arg(long)]
     * pub indexer_mh_url: Option<String>,
     */
    /// max cache size (bytes)
    #[arg(long)]
    pub max_cache_size: Option<u64>,
}
