use clap::{Args, Parser, Subcommand};

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
    /// Listen address
    #[arg(long)]
    pub listen_addr: Option<String>,
    /// Listen port
    #[arg(long)]
    pub listen_port: Option<u16>,
}
