mod backend;
mod cli;
mod config;
mod core;
mod resolve;
mod types;

use crate::{core::Server, resolve::CIDResolver};
use anyhow::{Context, Result};
use axum::{
    error_handling::HandleError,
    http::StatusCode,
    response::{IntoResponse, Response},
    Router,
};
use clap::Parser;
use cli::{Cli, Commands};
use config::{init_config, load_config};
use hyper::Client;
use moka::sync::Cache;
use std::{path::PathBuf, str::FromStr};
use tokio::{sync::oneshot::Sender, task::JoinHandle};
use tracing::Level;
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

    async fn handle_anyhow_error(err: anyhow::Error) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", err),
        )
            .into_response()
    }

    match command {
        Commands::Daemon(_) => {
            let client = Client::new();
            let resolver = CIDResolver::new(gateway_config.indexer.cid_url, client);
            let cache = Cache::new(10000);

            let app = Router::new().route_service(
                "/:cid",
                HandleError::new(Server::new(resolver, cache), handle_anyhow_error),
            );
            axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
                .serve(app.into_make_service())
                .await
                .unwrap();
        }
    }
    TelemetryConfig::teardown();
    Ok(())
}

async fn _graceful_shutdown(shutdown_tx: Sender<()>, worker: JoinHandle<()>) {
    tracing::info!("Gateway shutting down...");
    shutdown_tx
        .send(())
        .expect("Send shutdown signal successfully");
    worker.await.expect("Worker to shut down successfully");
}
