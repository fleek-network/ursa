mod config;

use crate::admin::config::get_config_handler;
use crate::config::CertConfig;
use crate::config::GatewayConfig;
use crate::config::ServerConfig;
use anyhow::{Context, Result};
use axum::{extract::Extension, routing::get, Router};
use axum_server::tls_rustls::RustlsConfig;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub async fn start_server(config: Arc<RwLock<GatewayConfig>>) -> Result<()> {
    let config_reader = config.clone();
    let GatewayConfig {
        cert: CertConfig {
            cert_path,
            key_path,
        },
        server: ServerConfig { addr, port },
        ..
    } = &(*(config_reader.read().await));

    let rustls_config = RustlsConfig::from_pem_file(&cert_path, &key_path)
        .await
        .with_context(|| {
            format!(
                "failed to init tls from:\ncert: {:?}:\npath:{:?}",
                cert_path, key_path
            )
        })?;

    let addr = SocketAddr::from((
        addr.parse::<Ipv4Addr>()
            .with_context(|| format!("failed to parse IPv4 with: {}", addr))?,
        *port,
    ));

    let app = Router::new()
        .route("/config", get(get_config_handler))
        .layer(Extension(config));

    info!("admin server listening on {}", addr);

    axum_server::bind_rustls(addr, rustls_config)
        .serve(app.into_make_service())
        .await
        .context("server stopped")?;

    Ok(())
}
