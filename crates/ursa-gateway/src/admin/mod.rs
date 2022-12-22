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
    cache::tlrfu::Tlrfu,
    config::{GatewayConfig, ServerConfig},
};

pub async fn start_server(
    config: Arc<RwLock<GatewayConfig>>,
    cache: Arc<RwLock<Tlrfu>>,
) -> Result<()> {
    let config_reader = config.clone();
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
            format!("failed to init tls from:\ncert: {cert_path:?}:\npath:{key_path:?}")
        })?;

    let addr = SocketAddr::from((
        addr.parse::<Ipv4Addr>()
            .with_context(|| format!("failed to parse IPv4 with: {addr}"))?,
        *port,
    ));

    let app = Router::new()
        .route("/config", get(get_config_handler))
        .route("/purge-cache", post(purge_cache_handler))
        .layer(Extension(config))
        .layer(Extension(cache));

    info!("admin server listening on {addr}");

    axum_server::bind_rustls(addr, rustls_config)
        .serve(app.into_make_service())
        .await
        .context("server stopped")?;

    Ok(())
}
