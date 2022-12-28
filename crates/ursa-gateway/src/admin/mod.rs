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
use axum_server::tls_rustls::RustlsConfig;
use tokio::sync::RwLock;
use tracing::info;

use crate::{
    admin::route::api::v1::{get::get_config_handler, post::purge_cache_handler},
    config::{GatewayConfig, ServerConfig},
    worker::cache::AdminCache,
};

pub async fn start<Cache: AdminCache>(
    config: Arc<RwLock<GatewayConfig>>,
    cache: Arc<RwLock<Cache>>,
) -> Result<()> {
    let config_reader = Arc::clone(&config);
    let GatewayConfig {
        admin_server:
            ServerConfig {
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

    axum_server::bind_rustls(addr, rustls_config)
        .serve(app.into_make_service())
        .await
        .context("Server stopped")?;

    Ok(())
}
