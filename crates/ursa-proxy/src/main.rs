use crate::core::ProxyCore;
use crate::{
    cli::{Cli, Commands},
    config::load_config,
};
use anyhow::Result;
use clap::Parser;
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
    ProxyCore::new(config).start_servers().await.unwrap();
    Ok(())
}
