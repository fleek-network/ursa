use crate::config::ProxyConfig;
use anyhow::{anyhow, Context, Result};
use axum::{routing::get, Router, Server};
use std::net::{IpAddr, SocketAddr};

pub async fn start_server(config: ProxyConfig) -> Result<()> {
    let server = config
        .server
        .first()
        .ok_or_else(|| anyhow!("No configuration found"))?;
    let addr = server.addr.clone();
    let port = server.port;
    let app = Router::new().route(
        "/",
        get(move || async move { format!("Sending request to {addr:?}:{port:?}") }),
    );
    let bind_addr = SocketAddr::from((
        server
            .listen_addr
            .clone()
            .unwrap_or("0.0.0.0".to_string())
            .parse::<IpAddr>()
            .context("Invalid binding address")?,
        server.listen_port.unwrap_or(8080),
    ));
    Server::bind(&bind_addr)
        .serve(app.into_make_service())
        .await
        .context("Server failed to start")
}
