mod handler;

use crate::config::ProxyConfig;
use crate::core::handler::proxy_pass;
use anyhow::{anyhow, Context, Result};
use axum::{routing::get, Extension, Router, Server};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

pub struct ServerConfig {
    pub addr: String,
    pub port: u16,
}

pub async fn start_server(config: ProxyConfig) -> Result<()> {
    let server = config
        .server
        .first()
        .ok_or_else(|| anyhow!("No configuration found"))?;
    let server_config = Arc::new(ServerConfig {
        addr: server.addr.clone(),
        port: server.port,
    });
    let app = Router::new()
        .route("/:cid", get(proxy_pass))
        .layer(Extension(server_config));
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
