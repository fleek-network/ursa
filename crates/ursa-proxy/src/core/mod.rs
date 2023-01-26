pub mod event;
mod handler;
mod worker;

use crate::{
    cache::{moka_cache::MokaCache, Cache, CacheWorker},
    config::ProxyConfig,
    core::handler::proxy_pass,
    core::{event::ProxyEvent, handler::proxy_pass_no_cache},
};
use anyhow::{bail, Context, Result};
use axum::{routing::get, Extension, Router, Server};
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};
use tokio::{spawn, task::JoinSet};
use tracing::info;

pub struct Proxy<C> {
    config: ProxyConfig,
    cache: C,
}

impl<C: Cache> Proxy<C> {
    pub fn new(config: ProxyConfig, cache: C) -> Self {
        Self { config, cache }
    }

    pub async fn start_with_cache_worker<W: CacheWorker<Command = C::Command>>(
        mut self,
        cache_worker: W,
    ) -> Result<()> {
        match self.cache.command_receiver().await {
            Some(cache_cmd_rx) => spawn(worker::start(cache_cmd_rx, cache_worker.clone())),
            None => bail!("Cache::command_receiver must return a command receiver"),
        };
        self.start().await
    }

    pub async fn start(self) -> Result<()> {
        let cache = self.cache.clone();
        spawn(async move {
            let duration_ms = Duration::from_millis(5 * 60 * 1000);
            loop {
                tokio::time::sleep(duration_ms).await;
                cache.handle_proxy_event(ProxyEvent::Timer).await;
            }
        });
        let mut servers = JoinSet::new();
        for server_config in self.config.server {
            let server_config = Arc::new(server_config);
            let mut app = Router::new().layer(Extension(server_config.clone()));
            if server_config.cache {
                app = app
                    .route("/*path", get(proxy_pass::<MokaCache>))
                    .layer(Extension(self.cache.clone()));
            } else {
                app = app.route("/*path", get(proxy_pass_no_cache))
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
