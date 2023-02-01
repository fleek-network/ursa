use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use ursa_proxy::{
    cache::moka_cache::MokaCache,
    cli::{Cli, Commands},
    config::load_config,
    core::Proxy,
};

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
