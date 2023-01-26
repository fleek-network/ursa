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
    let tlfru_config = config.tlrfu.clone().unwrap_or_default();
    let cache = TlrfuCache::new(
        tlfru_config.max_size,
        tlfru_config.ttl_buf,
        tlfru_config.stream_buf,
    );
    Proxy::new(config, cache.clone())
        .start_with_cache_worker(cache)
        .await
        .unwrap();
    Ok(())
}
