mod model;
mod route;

use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use anyhow::{Context, Result};
use axum::{extract::Extension, routing::get, Router};
use axum_server::tls_rustls::RustlsConfig;
use route::api::v1::get::get_block_handler;
use tokio::sync::RwLock;
use tracing::info;

use crate::{
    config::{GatewayConfig, ServerConfig},
    worker::cache::ServerCache,
};

pub async fn start<Cache: ServerCache>(
    config: Arc<RwLock<GatewayConfig>>,
    cache: Arc<RwLock<Cache>>,
) -> Result<()> {
    let config_reader = Arc::clone(&config);
    let GatewayConfig {
        server:
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
            format!("failed to init tls from: cert: {cert_path:?}: path: {key_path:?}")
        })?;

    let addr = SocketAddr::from((
        addr.parse::<Ipv4Addr>()
            .with_context(|| format!("failed to parse IPv4 with: {addr}"))?,
        *port,
    ));

    let app = Router::new()
        .route("/:cid", get(get_block_handler::<Cache>))
        .layer(Extension(config))
        .layer(Extension(cache));

    info!("reverse proxy listening on {addr}");

    axum_server::bind_rustls(addr, rustls_config)
        .serve(app.into_make_service())
        .await
        .context("server stopped")?;

    Ok(())
}
