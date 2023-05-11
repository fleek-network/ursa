extern crate core;

mod balance;
mod cli;
mod config;
mod resolver;
mod server;
mod util;

use std::{path::PathBuf, str::FromStr, sync::Arc};

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Commands};
use config::{init_config, load_config};
use tokio::{
    select,
    signal::{
        ctrl_c,
        unix::{signal, SignalKind},
    },
    spawn,
    sync::{
        mpsc::{self},
        oneshot::{self, Sender},
        RwLock,
    },
    task::JoinHandle,
};
use tracing::{error, info, info_span, Instrument, Level};
use ursa_telemetry::TelemetryConfig;

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

    TelemetryConfig::new("ursa-gateway")
        .with_log_level(log_level.as_str())
        .with_pretty_log()
        .with_jaeger_tracer()
        .init()?;

    match command {
        Commands::Daemon(opts) => {
            let _s = info_span!("Daemon start").entered();

            // sync
            gateway_config.merge_daemon_opts(opts);

            let server_config = Arc::new(RwLock::new(gateway_config));

            let (shutdown_tx, shutdown_rx) = oneshot::channel();

            let (server_worker, mut server_worker_signal_rx) = {
                let (signal_tx, signal_rx) = mpsc::channel(1);
                let worker = async move {
                    if let Err(e) = server::start(server_config, shutdown_rx).await {
                        error!("[Server]: {e:?}");
                        signal_tx.send(()).await.expect("Send signal successfully");
                    };
                    info!("Server stopped");
                };
                (
                    spawn(worker.instrument(info_span!("Server worker"))),
                    signal_rx,
                )
            };

            #[cfg(unix)]
            let terminate = async {
                signal(SignalKind::terminate())
                    .expect("Failed to install signal handler")
                    .recv()
                    .await;
            };

            #[cfg(not(unix))]
            let terminate = std::future::pending::<()>();

            select! {
                _ = ctrl_c() => graceful_shutdown(shutdown_tx, server_worker).await,
                _ = terminate => graceful_shutdown(shutdown_tx, server_worker).await,
                _ = server_worker_signal_rx.recv() => graceful_shutdown(shutdown_tx, server_worker).await,
            }
            info!("Gateway shut down successfully")
        }
    }
    TelemetryConfig::teardown();
    Ok(())
}

async fn graceful_shutdown(shutdown_tx: Sender<()>, worker: JoinHandle<()>) {
    info!("Gateway shutting down...");
    shutdown_tx
        .send(())
        .expect("Send shutdown signal successfully");
    worker.await.expect("Worker to shut down successfully");
}
