mod route;

use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use axum::{
    extract::Extension,
    routing::{get, post},
    Router,
};
use axum_server::{tls_rustls::RustlsConfig, Handle};
use route::api::v1::{get::get_config_handler, post::purge_cache_handler};
use tokio::{
    select, spawn,
    sync::{broadcast::Receiver, RwLock},
};
use tracing::info;

use crate::{
    config::{AdminConfig, GatewayConfig},
    worker::cache::AdminCache,
};

pub async fn start<Cache: AdminCache>(
    config: Arc<RwLock<GatewayConfig>>,
    cache: Arc<RwLock<Cache>>,
    admin_shutdown_rx: Receiver<()>,
) -> Result<()> {
    let config_reader = Arc::clone(&config);
    let GatewayConfig {
        admin_server:
            AdminConfig {
                addr,
                port,
                cert_path,
                key_path,
            },
        ..
    } = &(*config_reader.read().await);

    let rustls_config = RustlsConfig::from_pem_file(&cert_path, &key_path)
        .await
        .with_context(|| {
            format!("Failed to init tls from: cert: {cert_path:?}: path:{key_path:?}")
        })?;

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
    spawn(graceful_shutdown(handle.clone(), admin_shutdown_rx));

    axum_server::bind_rustls(addr, rustls_config)
        .handle(handle)
        .serve(app.into_make_service())
        .await
        .context("Failed to start admin server")?;

    Ok(())
}

async fn graceful_shutdown(handle: Handle, mut admin_shutdown_rx: Receiver<()>) {
    select! {
        _ = admin_shutdown_rx.recv() => {
            loop {
                if handle.connection_count() == 0 {
                    handle.shutdown();
                    break;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
                info!("Admin server alive connections: {}", handle.connection_count());
            }
        }
    }
}
