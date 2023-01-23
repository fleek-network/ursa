mod route;

use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use anyhow::{Context, Result};
use axum::{
    extract::Extension,
    routing::{get, post},
    Router,
};
use axum_server::Handle;
use route::api::v1::{get::get_config_handler, post::purge_cache_handler};
use tokio::{
    select, spawn,
    sync::{broadcast::Receiver, RwLock},
};
use tracing::info;

use crate::{
    config::{AdminConfig, GatewayConfig},
    worker::cache::admin::AdminCache,
};

pub async fn start<Cache: AdminCache>(
    config: Arc<RwLock<GatewayConfig>>,
    cache: Arc<RwLock<Cache>>,
    shutdown_rx: Receiver<()>,
) -> Result<()> {
    let config_reader = Arc::clone(&config);
    let GatewayConfig {
        admin_server: AdminConfig { addr, port },
        ..
    } = &(*config_reader.read().await);

    let addr = SocketAddr::from((
        addr.parse::<Ipv4Addr>()
            .with_context(|| format!("Failed to parse IPv4 with: {addr}"))?,
        *port,
    ));

    let app = Router::new()
        .route("/config", get(get_config_handler))
        .route("/purge-cache", post(purge_cache_handler::<Cache>))
        .layer(Extension(config))
        .layer(Extension(cache));

    info!("Admin server listening on {addr}");

    let handle = Handle::new();
    spawn(graceful_shutdown(handle.clone(), shutdown_rx));

    axum_server::bind(addr)
        .handle(handle)
        .serve(app.into_make_service())
        .await
        .context("Failed to start admin server")?;

    Ok(())
}

async fn graceful_shutdown(handle: Handle, mut shutdown_rx: Receiver<()>) {
    select! {
        _ = shutdown_rx.recv() => {
            handle.shutdown();
        }
    }
}
