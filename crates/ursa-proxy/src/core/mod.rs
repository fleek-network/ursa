use anyhow::{Context, Result};
use axum::{routing::get, Router, Server};
use std::net::{IpAddr, SocketAddr};

pub async fn start_server() -> Result<()> {
    let addr = SocketAddr::from((IpAddr::from([0, 0, 0, 0]), 8080));
    let app = Router::new().route("/", get(|| async { "Hello, world! - Proxy" }));
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .context("Server failed to start")
}
