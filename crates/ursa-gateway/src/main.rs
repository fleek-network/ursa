mod cli;
mod config;
mod server;

use crate::config::{init_config, load_config};
use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Commands};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let Cli { config, command } = Cli::parse();

    let config_path = PathBuf::from(config);

    init_config(&config_path)
        .with_context(|| format!("failed to init config from: {:?}", config_path))?;

    let mut gateway_config = load_config(&config_path)
        .with_context(|| format!("failed to load config from: {:?}", config_path))?;

    match command {
        Commands::Daemon(config) => {
            gateway_config.merge(config);
            server::start_server(gateway_config).await?;
        }
    }

    Ok(())
}
