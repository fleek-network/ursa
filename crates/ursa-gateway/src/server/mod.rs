mod model;
mod route;

use crate::config::{CertConfig, GatewayConfig, ServerConfig};
use anyhow::{Context, Result};
use axum::{extract::Extension, routing::get, Router};
use axum_server::tls_rustls::RustlsConfig;
use hyper::client::HttpConnector;
use hyper::Body;
use hyper_tls::HttpsConnector;
use route::api::v1::get::get_block_handler;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use tracing::info;

type Client = hyper::client::Client<HttpsConnector<HttpConnector>, Body>;

pub async fn start_server(gateway_config: GatewayConfig) -> Result<()> {
    let GatewayConfig {
        cert:
            CertConfig {
                cert_path: tls_cert_path,
                key_path: tls_key_path,
            },
        server: ServerConfig { addr, port },
        ..
    } = gateway_config.clone();

    let rustls_config = RustlsConfig::from_pem_file(&tls_cert_path, &tls_key_path)
        .await
        .with_context(|| {
            format!(
                "failed to init tls from:\ncert: {:?}:\npath:{:?}",
                tls_cert_path, tls_key_path
            )
        })?;

    let client = hyper::Client::builder().build::<_, Body>(HttpsConnector::new());

    let app = Router::new()
        .route("/:cid", get(get_block_handler))
        .layer(Extension((client, gateway_config)));

    let addr = SocketAddr::from((
        addr.parse::<Ipv4Addr>()
            .with_context(|| format!("failed to parse IPv4 with: {}", addr))?,
        port,
    ));

    info!("reverse proxy listening on {}", addr);

    axum_server::bind_rustls(addr, rustls_config)
        .serve(app.into_make_service())
        .await
        .context("server stopped")?;

    Ok(())
}
