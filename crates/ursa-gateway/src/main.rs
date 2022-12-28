mod admin;
mod cache;
mod cli;
mod config;
mod indexer;
mod server;
mod util;
mod worker;

use std::{path::PathBuf, str::FromStr, sync::Arc};

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Commands};
use hyper::Body;
use hyper_tls::HttpsConnector;
use indexer::Indexer;
use tokio::{
    sync::{mpsc, RwLock},
    task,
};
use tracing::{error, Level};
use worker::cache::Cache;

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

            let indexer = Arc::new(Indexer::new(
                String::from(&gateway_config.indexer.cid_url),
                hyper::Client::builder().build::<_, Body>(HttpsConnector::new()),
            ));

            let (worker_tx, worker_rx) = mpsc::unbounded_channel();
            let cache = Arc::new(RwLock::new(Cache::new(
                gateway_config.cache.max_size,
                gateway_config.cache.ttl_buf as u128 * 1_000_000, // ms to ns
                worker_tx,
            )));
            let server_cache = Arc::clone(&cache);
            let admin_cache = Arc::clone(&server_cache);

            let server_config = Arc::new(RwLock::new(gateway_config));
            let admin_config = Arc::clone(&server_config);

            task::spawn(async move {
                if let Err(e) = server::start(server_config, server_cache).await {
                    error!("[Gateway server]: {:?}", e);
                };
            });

            task::spawn(async move {
                if let Err(e) = admin::start(admin_config, admin_cache).await {
                    error!("[Admin server]: {:?}", e);
                };
            });

            worker::start(worker_rx, cache, indexer).await;
        }
    }

    Ok(())
}
