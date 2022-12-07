mod config;
mod routes;

use crate::config::GatewayConfig;
use crate::routes::api::v1::get::get_block_handler;
use crate::routes::api::v1::put::put_file_handler;
use axum::{
    extract::Extension,
    routing::{get, post},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use hyper::{client::HttpConnector, Body};
use std::net::SocketAddr;
use tracing::info;

type Client = hyper::client::Client<HttpConnector, Body>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = GatewayConfig::default();

    let rustls_config =
        RustlsConfig::from_pem_file(config.cert_config.cert_path, config.cert_config.key_path)
            .await
            .unwrap();

    let app = Router::new()
        .route("/get-cid", get(get_block_handler))
        .route("/put-file", post(put_file_handler))
        .layer(Extension((Client::new(), GatewayConfig::default())));

    let addr = SocketAddr::from((
        config.server.addr.parse::<std::net::Ipv4Addr>().unwrap(),
        config.server.port,
    ));

    info!("reverse proxy listening on {}", addr);

    axum_server::bind_rustls(addr, rustls_config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
