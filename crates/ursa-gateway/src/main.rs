mod config;
mod routes;

use crate::config::GatewayConfig;
use crate::routes::api::v1::get::get_block_handler;
use crate::routes::api::v1::put::{put_car_handler, put_file_handler};
use axum::{
    extract::Extension,
    http::{uri::Uri, Request, Response},
    routing::{get, post},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use hyper::{client::HttpConnector, Body};
use std::{convert::TryFrom, net::SocketAddr};
use tracing::info;

type Client = hyper::client::Client<HttpConnector, Body>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    tokio::spawn(server(GatewayConfig::default()));

    let config = GatewayConfig::default();

    let rustls_config =
        RustlsConfig::from_pem_file(config.cert_config.cert_path, config.cert_config.key_path)
            .await
            .unwrap();

    let app = Router::new()
        .route("/get-cid", get(forward_handler))
        .route("/put-car", post(forward_handler))
        .route("/put-file", post(forward_handler))
        .layer(Extension((Client::new(), GatewayConfig::default())));

    let addr = SocketAddr::from((
        config
            .reverse_proxy
            .addr
            .parse::<std::net::Ipv4Addr>()
            .unwrap(),
        config.reverse_proxy.port,
    ));

    info!("reverse proxy listening on {}", addr);

    axum_server::bind_rustls(addr, rustls_config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn forward_handler(
    Extension((client, config)): Extension<(Client, GatewayConfig)>,
    mut req: Request<Body>,
) -> Response<Body> {
    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(path);

    let uri = format!(
        "http://{}:{}{}",
        config.server.addr, config.server.port, path_query
    );

    *req.uri_mut() = Uri::try_from(uri).unwrap();

    client.request(req).await.unwrap()
}

async fn server(config: GatewayConfig) {
    let app = Router::new()
        .route("/get-cid", get(get_block_handler))
        .route("/put-car", post(put_car_handler))
        .route("/put-file", post(put_file_handler));

    let addr = SocketAddr::from((
        config.server.addr.parse::<std::net::Ipv4Addr>().unwrap(),
        config.server.port,
    ));

    info!("server listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
