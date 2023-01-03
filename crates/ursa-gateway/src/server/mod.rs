mod model;
mod route;

use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use axum::{extract::Extension, routing::get, Router};
use axum_server::{tls_rustls::RustlsConfig, Handle};
use route::api::v1::get::get_car_handler;
use tokio::{
    select, spawn,
    sync::{broadcast::Receiver, RwLock},
};
use tracing::info;

use crate::{
    config::{GatewayConfig, ServerConfig},
    worker::cache::ServerCache,
};

pub async fn start<Cache: ServerCache>(
    config: Arc<RwLock<GatewayConfig>>,
    cache: Arc<RwLock<Cache>>,
    shutdown_rx: Receiver<()>,
) -> Result<()> {
    let config_reader = Arc::clone(&config);
    let GatewayConfig {
        server:
            ServerConfig {
                addr,
                port,
                cert_path,
                key_path,
                ..
            },
        ..
    } = &(*config_reader.read().await);

    let rustls_config = RustlsConfig::from_pem_file(&cert_path, &key_path)
        .await
        .with_context(|| {
            format!("Failed to init tls from: cert: {cert_path:?}: path: {key_path:?}")
        })?;

    let addr = SocketAddr::from((
        addr.parse::<Ipv4Addr>()
            .with_context(|| format!("Failed to parse IPv4 with: {addr}"))?,
        *port,
    ));

    let app = Router::new()
        .route("/:cid", get(get_car_handler::<Cache>))
        .layer(Extension(config))
        .layer(Extension(cache));

    info!("Reverse proxy listening on {addr}");

    let handle = Handle::new();
    spawn(graceful_shutdown(handle.clone(), shutdown_rx));

    axum_server::bind_rustls(addr, rustls_config)
        .handle(handle)
        .serve(app.into_make_service())
        .await
        .context("Failed to start server")?;

    Ok(())
}

async fn graceful_shutdown(handle: Handle, mut shutdown_rx: Receiver<()>) {
    select! {
        _ = shutdown_rx.recv() => {
            loop {
                if handle.connection_count() == 0 {
                    handle.shutdown();
                    break;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
                info!("Server alive connections: {}", handle.connection_count());
            }
        }
    }
}
