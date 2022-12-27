mod model;
mod route;

use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use anyhow::{Context, Result};
use axum::{extract::Extension, routing::get, Router};
use axum_server::tls_rustls::RustlsConfig;
use hyper::Body;
use hyper_tls::HttpsConnector;
use route::api::v1::get::get_block_handler;
use tokio::sync::{mpsc::UnboundedSender, RwLock};
use tracing::info;

use crate::{
    cache::Tlrfu,
    config::{GatewayConfig, ServerConfig},
};

pub async fn start(
    config: Arc<RwLock<GatewayConfig>>,
    cache: Arc<RwLock<Tlrfu>>,
    worker_sender: Arc<UnboundedSender<String>>,
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
            format!("failed to init tls from:\ncert: {cert_path:?}:\npath: {key_path:?}")
        })?;

    let addr = SocketAddr::from((
        addr.parse::<Ipv4Addr>()
            .with_context(|| format!("failed to parse IPv4 with: {addr}"))?,
        *port,
    ));

    let client = Arc::new(hyper::Client::builder().build::<_, Body>(HttpsConnector::new()));

    let app = Router::new()
        .route("/:cid", get(get_block_handler))
        .layer(Extension(client))
        .layer(Extension(config))
        .layer(Extension(cache))
        .layer(Extension(worker_sender));

    info!("reverse proxy listening on {addr}");

    axum_server::bind_rustls(addr, rustls_config)
        .serve(app.into_make_service())
        .await
        .context("server stopped")?;

    Ok(())
}
