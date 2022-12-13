mod route;

use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};

use anyhow::{Context, Result};
use axum::{extract::Extension, routing::get, Router};
use axum_server::tls_rustls::RustlsConfig;
use tokio::sync::RwLock;
use tracing::info;

use crate::{
    admin::route::api::v1::get::get_config_handler,
    config::{GatewayConfig, ServerConfig},
};

pub async fn start_server(config: Arc<RwLock<GatewayConfig>>) -> Result<()> {
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
