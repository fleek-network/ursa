mod cache;
pub mod cli;
mod config;
mod core;

use crate::{
    cli::{Cli, Commands},
    config::load_config,
    {cache::moka_cache::MokaCache, core::Proxy},
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
    let moka_config = config.moka.clone().unwrap_or_default();
    let cache = MokaCache::new(moka_config.stream_buf);
    Proxy::new(config, cache.clone()).start().await
}
