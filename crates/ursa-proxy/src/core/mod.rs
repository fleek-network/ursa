use anyhow::{Context, Result};
use axum::{routing::get, Router, Server};
use std::net::SocketAddr;

pub async fn start_server(bind_addr: &SocketAddr) -> Result<()> {
    let app = Router::new().route("/", get(|| async { "Hello, world! - Proxy" }));
    Server::bind(bind_addr)
        .serve(app.into_make_service())
        .await
        .context("Server failed to start")
}
