mod cache;
pub mod cli;
mod config;
mod core;

use crate::{
    cli::{Cli, Commands},
    config::load_config,
    {cache::tlrfu_cache::TlrfuCache, core::Proxy},
};
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let Cli {
        command: Commands::Daemon(opts),
    } = Cli::parse();
    let config = load_config(&opts.config.parse::<PathBuf>()?)?;
    let cache = TlrfuCache::new(200_000_000, 5 * 60 * 1000, 2_000_000);
    Proxy::new(config, cache.clone())
        .start_with_cache_worker(cache)
        .await
        .unwrap();
    Ok(())
}
