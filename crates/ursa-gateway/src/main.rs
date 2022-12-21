mod admin;
mod cache;
mod cli;
mod config;
mod indexer;
mod server;
mod worker;

use std::{path::PathBuf, str::FromStr, sync::Arc};

use anyhow::{Context, Result};
use cache::tlrfu::TLRFUCache;
use clap::Parser;
use cli::{Cli, Commands};
use tokio::{sync::RwLock, task};
use tracing::{error, Level};

use crate::config::{init_config, load_config};

#[tokio::main]
async fn main() -> Result<()> {
    let Cli {
        log,
        config,
        command,
    } = Cli::parse();

    let config_path = PathBuf::from(config);
    init_config(&config_path)
        .with_context(|| format!("failed to init config from: {config_path:?}"))?;
    let mut gateway_config = load_config(&config_path)
        .with_context(|| format!("failed to load config from: {config_path:?}"))?;

    // sync
    gateway_config.merge_log_level(log);

    // override log level if present in cli opts
    let log_level = log.unwrap_or(Level::from_str(&gateway_config.log_level)?);
    tracing_subscriber::fmt().with_max_level(log_level).init();

    match command {
        Commands::Daemon(opts) => {
            // sync
            gateway_config.merge_daemon_opts(opts);

            let server_cache = Arc::new(RwLock::new(TLRFUCache::new(
                gateway_config.cache.max_size,
                0,
            )));
            let admin_cache = server_cache.clone();

            let server_config = Arc::new(RwLock::new(gateway_config));
            let admin_config = server_config.clone();

            task::spawn(async {
                if let Err(e) = admin::start_server(admin_config, admin_cache).await {
                    error!("[admin server] - {:?}", e);
                };
            });

            server::start_server(server_config, server_cache).await?;
        }
    }

    Ok(())
}
