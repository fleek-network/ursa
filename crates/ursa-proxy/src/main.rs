use crate::cli::{Cli, Commands};
use anyhow::{Context, Result};
use clap::Parser;
use std::net::{IpAddr, SocketAddr};

mod cli;
mod config;
mod core;

#[tokio::main]
async fn main() -> Result<()> {
    let Cli {
        command: Commands::Daemon(cmd_opts),
    } = Cli::parse();

    let addr = SocketAddr::from((
        cmd_opts
            .listen_addr
            .unwrap_or("0.0.0.0".to_string())
            .parse::<IpAddr>()
            .context("Invalid binding address")?,
        cmd_opts.listen_port.unwrap_or(8080),
    ));
    core::start_server(&addr).await.unwrap();
    Ok(())
}
