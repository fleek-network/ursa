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
    /// request time out (ms)
    #[arg(long)]
    pub request_timeout: Option<u64>,
    /// concurrency limit
    #[arg(long)]
    pub concurrency_limit: Option<u32>,
    /// tls cert path
    #[arg(long)]
    pub tls_cert_path: Option<PathBuf>,
    /// tls key path
    #[arg(long)]
    pub tls_key_path: Option<PathBuf>,
    /// server stream buffer
    #[arg(long)]
    pub server_stream_buffer: Option<u64>,
    /// admin port
    #[arg(long)]
    pub admin_port: Option<u16>,
    /// admin address
    #[arg(long)]
    pub admin_addr: Option<String>,
    /// indexer cid url
    #[arg(long)]
    pub indexer_cid_url: Option<String>,
    /// max cache capacity (number of entries)
    #[arg(long)]
    pub cache_max_capacity: Option<u64>,
    /// cache ttl (ms)
    #[arg(long)]
    pub cache_time_to_live: Option<u64>,
    /// time to idle cache interval (ms)
    #[arg(long)]
    pub cache_time_to_idle: Option<u64>,
    /// path to maxmind db (city)
    #[arg(long)]
    pub maxminddb: Option<PathBuf>,
}
