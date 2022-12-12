use crate::config::DEFAULT_URSA_GATEWAY_CONFIG_PATH;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[arg(long, default_value_t = format!("{}/{}", env!("HOME"), DEFAULT_URSA_GATEWAY_CONFIG_PATH))]
    pub config: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run gateway daemon
    Daemon,
}
