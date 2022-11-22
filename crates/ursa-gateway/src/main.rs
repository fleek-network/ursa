mod config;

use crate::config::GatewayConfig;
use axum::{
    extract::{Extension, Query},
    http::{uri::Uri, Request, Response},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use hyper::StatusCode;
use hyper::{client::HttpConnector, Body};
use jsonrpc_v2::Error::{Full, Provided};
use serde_json::{json, Value};
use std::{convert::TryFrom, net::SocketAddr};
use tracing::info;
use ursa_rpc_client::functions::{get_block, put_car, put_file};
use ursa_rpc_server::api::{NetworkGetParams, NetworkPutCarParams, NetworkPutFileParams};

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
        .route("/get-cid", get(handler))
        .route("/put-car", post(handler))
        .route("/put-file", post(handler))
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

async fn handler(
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

async fn get_block_handler(Query(params): Query<NetworkGetParams>) -> impl IntoResponse {
    match get_block(params).await {
        Ok(res) => (StatusCode::OK, Json(json!(res))),
        Err(err) => handle_error(err),
    }
}

async fn put_car_handler(Json(params): Json<NetworkPutCarParams>) -> impl IntoResponse {
    match put_car(params).await {
        Ok(res) => (StatusCode::OK, Json(json!(res))),
        Err(err) => handle_error(err),
    }
}

async fn put_file_handler(Json(params): Json<NetworkPutFileParams>) -> impl IntoResponse {
    match put_file(params).await {
        Ok(res) => (StatusCode::OK, Json(json!(res))),
        Err(err) => handle_error(err),
    }
}

fn handle_error(err: jsonrpc_v2::Error) -> (StatusCode, Json<Value>) {
    match err {
        Full { code: 200, .. } => (StatusCode::OK, Json(json!(err))),
        Provided { code: 200, .. } => (StatusCode::OK, Json(json!(err))),
        Full { .. } => (StatusCode::BAD_REQUEST, Json(json!(err))),
        Provided { .. } => (StatusCode::BAD_REQUEST, Json(json!(err))),
    }
}
