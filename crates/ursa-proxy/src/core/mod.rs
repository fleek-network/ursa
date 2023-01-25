pub mod event;
mod handler;
mod worker;

use crate::cache::moka_cache::MokaCache;
use crate::{config::ProxyConfig, core::handler::proxy_pass};
use anyhow::{Context, Result};
use axum::{routing::get, Extension, Router, Server};
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::task::JoinSet;
use tracing::info;

// We want to generically pass_proxy
// We want to allow users to pass their own logic instead of generic pass_proxy
// We want each of the handlers above to get the config needed to make decisions
pub struct ProxyCore {
    config: ProxyConfig,
    cache: Arc<MokaCache>,
}

impl ProxyCore {
    pub fn new(config: ProxyConfig) -> Self {
        Self {
            config,
            cache: Arc::new(MokaCache::new()),
        }
    }

    pub async fn start(self) -> Result<()> {
        Ok(())
    }

    pub async fn start_servers(self) -> Result<()> {
        let mut servers = JoinSet::new();
        for server_config in self.config.server {
            let server_config = Arc::new(server_config);
            let app = Router::new()
                .route("/*path", get(proxy_pass::<Arc<MokaCache>>))
                .layer(Extension(server_config.clone()))
                .layer(Extension(self.cache.clone()));
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