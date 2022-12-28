mod admin;
mod cache;
mod cli;
mod config;
mod resolver;
mod server;
mod util;
mod worker;

use std::{path::PathBuf, str::FromStr, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Commands};
use config::{init_config, load_config};
use hyper::Body;
use hyper_tls::HttpsConnector;
use resolver::Resolver;
use tokio::{
    sync::{mpsc, RwLock},
    task,
};
use tracing::{error, info, Level};
use worker::cache::Cache;

#[tokio::main]
async fn main() -> Result<()> {
    let Cli {
        log,
        config,
        command,
    } = Cli::parse();

    let config_path = PathBuf::from(config);
    init_config(&config_path)
        .with_context(|| format!("Failed to init config from: {config_path:?}"))?;
    let mut gateway_config = load_config(&config_path)
        .with_context(|| format!("Failed to load config from: {config_path:?}"))?;

    // sync
    gateway_config.merge_log_level(log);

    // override log level if present in cli opts
    let log_level = log.unwrap_or(Level::from_str(&gateway_config.log_level)?);
    tracing_subscriber::fmt().with_max_level(log_level).init();

    match command {
        Commands::Daemon(opts) => {
            // sync
            gateway_config.merge_daemon_opts(opts);

            let worker_tll_interval = gateway_config.worker.ttl_interval;

            let resolver = Arc::new(Resolver::new(
                String::from(&gateway_config.indexer.cid_url),
                hyper::Client::builder().build::<_, Body>(HttpsConnector::new()),
            ));

            let (worker_tx, worker_rx) = mpsc::unbounded_channel();
            let cache = Arc::new(RwLock::new(Cache::new(
                gateway_config.cache.max_size,
                gateway_config.cache.ttl_buf as u128 * 1_000_000, // ms to ns
                worker_tx.clone(),                                // cache command producer
            )));
            let server_cache = Arc::clone(&cache);
            let admin_cache = Arc::clone(&server_cache);

            let server_config = Arc::new(RwLock::new(gateway_config));
            let admin_config = Arc::clone(&server_config);

            task::spawn(async move {
                if let Err(e) = server::start(server_config, server_cache).await {
                    error!("[Gateway server]: {e}");
                };
            });

            task::spawn(async move {
                if let Err(e) = admin::start(admin_config, admin_cache).await {
                    error!("[Admin server]: {e}");
                };
            });

            task::spawn(async move {
                let duration_ms = Duration::from_millis(worker_tll_interval);
                info!("[Cache TTL Worker]: Interval: {duration_ms:?}");
                loop {
                    tokio::time::sleep(duration_ms).await;
                    if let Err(e) = worker_tx.send(worker::cache::WorkerCacheCommand::TtlCleanUp) {
                        error!("[Cache TTL Worker]: {e}");
                    }
                }
            });

            worker::start(worker_rx, cache, resolver).await;
        }
    }

    Ok(())
}
