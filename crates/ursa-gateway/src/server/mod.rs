mod model;
mod route;

use crate::config::GatewayConfig;
use anyhow::{Context, Result};
use axum::{extract::Extension, routing::get, Router};
use axum_server::tls_rustls::RustlsConfig;
use hyper::client::HttpConnector;
use hyper::Body;
use hyper_tls::HttpsConnector;
use route::api::v1::get::get_block_handler;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

type Client = hyper::client::Client<HttpsConnector<HttpConnector>, Body>;

pub async fn start_server(config: GatewayConfig) -> Result<()> {
    let rustls_config = RustlsConfig::from_pem_file(&config.cert.cert_path, &config.cert.key_path)
        .await
        .with_context(|| {
            format!(
                "failed to init tls from:\ncert: {:?}:\npath:{:?}",
                config.cert.cert_path, config.cert.key_path
            )
        })?;

    let client = hyper::Client::builder().build::<_, Body>(HttpsConnector::new());

    let addr = SocketAddr::from((
        config
            .server
            .addr
            .parse::<Ipv4Addr>()
            .with_context(|| format!("failed to parse IPv4 with: {}", config.server.addr))?,
        config.server.port,
    ));

    let app = Router::new()
        .route("/:cid", get(get_block_handler))
        .layer(Extension((client, Arc::new(config))));

    info!("reverse proxy listening on {}", addr);

    axum_server::bind_rustls(addr, rustls_config)
        .serve(app.into_make_service())
        .await
        .context("server stopped")?;

    Ok(())
}
