use crate::cli::{Cli, Commands};
use crate::config::load_config;
use anyhow::{Context, Result};
use clap::Parser;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

mod cli;
mod config;
mod core;

#[tokio::main]
async fn main() -> Result<()> {
    let Cli {
        command: Commands::Daemon(opts),
    } = Cli::parse();

    let config = load_config(&opts.config.parse::<PathBuf>()?)?;
    core::start_server(config).await.unwrap();
    Ok(())
}
