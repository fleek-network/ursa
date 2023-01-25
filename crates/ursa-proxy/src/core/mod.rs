pub mod event;
mod handler;
mod worker;

use crate::cache::moka_cache::MokaCache;
use crate::cache::CacheClient;
use crate::{config::ProxyConfig, core::handler::proxy_pass};
use anyhow::{Context, Result};
use axum::{routing::get, Extension, Router, Server};
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::spawn;
use tokio::task::JoinSet;
use tracing::info;

pub struct Proxy<C> {
    config: ProxyConfig,
    cache: C,
}

impl<C: CacheClient> Proxy<C> {
    pub fn new(config: ProxyConfig, cache: C) -> Self {
        Self { config, cache }
    }

    pub async fn start(mut self) -> Result<()> {
        let cache_rx = self.cache.command_receiver().await;
        spawn(worker::start(cache_rx, self.cache.clone()));
        let mut servers = JoinSet::new();
        for server_config in self.config.server {
            let server_config = Arc::new(server_config);
            let mut app = Router::new().layer(Extension(server_config.clone()));
            if server_config.cache {
                app = app
                    .route("/*path", get(proxy_pass::<Arc<MokaCache>>))
                    .layer(Extension(self.cache.clone()));
            }
            let bind_addr = SocketAddr::from((
                server_config
                    .listen_addr
                    .clone()
                    .unwrap_or_else(|| "0.0.0.0".to_string())
                    .parse::<IpAddr>()
                    .context("Invalid binding address")?,
                server_config.listen_port.unwrap_or(0),
            ));
            info!("Listening on {bind_addr:?}");
            servers.spawn(Server::bind(&bind_addr).serve(app.into_make_service()));
        }
        // TODO: Implement safe cancel.
        while servers.join_next().await.is_some() {}
        Ok(())
    }
}
