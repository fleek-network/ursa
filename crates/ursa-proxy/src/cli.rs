use clap::{Args, Parser, Subcommand};

use crate::config::DEFAULT_URSA_PROXY_CONFIG_PATH;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run proxy daemon
    Daemon(DaemonCmdOpts),
}

#[derive(Args)]
pub struct DaemonCmdOpts {
    /// Config path
    #[arg(long, default_value_t = format!("{}/{}", env!("HOME"), DEFAULT_URSA_PROXY_CONFIG_PATH))]
    pub config: String,
}
