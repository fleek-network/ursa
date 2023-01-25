use crate::cache::moka_cache::MokaCache;
use crate::core::Proxy;
use crate::{
    cli::{Cli, Commands},
    config::load_config,
};
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

mod cache;
pub mod cli;
mod config;
mod core;

#[tokio::main]
async fn main() -> Result<()> {
    let Cli {
        command: Commands::Daemon(opts),
    } = Cli::parse();
    let config = load_config(&opts.config.parse::<PathBuf>()?)?;
    let cache = MokaCache::new(100_000);
    Proxy::new(config, cache).start().await.unwrap();
    Ok(())
}
