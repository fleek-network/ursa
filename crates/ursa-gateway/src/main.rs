mod config;
mod routes;

use crate::config::{init_config, load_config, GatewayConfig};
use crate::routes::api::v1::get::get_block_handler;
use axum::{extract::Extension, routing::get, Router};
use axum_server::tls_rustls::RustlsConfig;
use hyper::{client::HttpConnector, Body};
use std::net::SocketAddr;
use tracing::info;

type Client = hyper::client::Client<HttpConnector, Body>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    init_config();

    let config = load_config();

    let rustls_config =
        RustlsConfig::from_pem_file(config.cert_config.cert_path, config.cert_config.key_path)
            .await
            .expect("problem starting server");

    let app = Router::new()
        .route("/:cid", get(get_block_handler))
        .layer(Extension((Client::new(), GatewayConfig::default())));

    let addr = SocketAddr::from((
        config
            .server
            .addr
            .parse::<std::net::Ipv4Addr>()
            .expect("problem parse ipv4"),
        config.server.port,
    ));

    info!("reverse proxy listening on {}", addr);

    axum_server::bind_rustls(addr, rustls_config)
        .serve(app.into_make_service())
        .await
        .expect("problem start server");
}
