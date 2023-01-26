use crate::cache::moka_cache::MokaCache;
use crate::cache::tlrfu_cache::TCache;
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
    let cache = TCache::new(200_000_000, 5 * 60 * 1000, 2_000_000);
    Proxy::new(config, cache.clone())
        .start_with_cache_worker(cache)
        .await
        .unwrap();
    Ok(())
}
