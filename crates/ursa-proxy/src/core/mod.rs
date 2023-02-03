pub mod event;
mod handler;
mod worker;

use crate::{
    cache::{Cache, CacheWorker},
    config::ProxyConfig,
    core::{event::ProxyEvent, handler::proxy_pass},
};
use anyhow::{bail, Context, Result};
use axum::{routing::get, Extension, Router};
use axum_server::{tls_rustls::RustlsConfig, Handle};
use std::{
    io::Result as IOResult,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};
use tokio::{select, spawn, sync::mpsc::Receiver, task::JoinSet};
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
        shutdown_rx: Receiver<()>,
    ) -> Result<()> {
        match self.cache.command_receiver().await {
            Some(cache_cmd_rx) => spawn(worker::start(cache_cmd_rx, cache_worker.clone())),
            None => bail!("Cache::command_receiver must return a command receiver"),
        };
        self.start(shutdown_rx).await
    }

    pub async fn start(self, mut shutdown_rx: Receiver<()>) -> Result<()> {
        let mut workers = JoinSet::new();
        let cache = self.cache.clone();
        workers.spawn(async move {
            let duration_ms = Duration::from_millis(5 * 60 * 1000);
            loop {
                tokio::time::sleep(duration_ms).await;
                cache.handle_proxy_event(ProxyEvent::Timer).await;
            }
        });

        let handle = Handle::new();
        for server_config in self.config.server {
            let server_config = Arc::new(server_config);
            let app = Router::new()
                .route("/*path", get(proxy_pass::<C>))
                .layer(Extension(self.cache.clone()))
                .layer(Extension(server_config.clone()));
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
            let rustls_config =
                RustlsConfig::from_pem_file(&server_config.cert_path, &server_config.key_path)
                    .await
                    .unwrap();
            workers.spawn(
                axum_server::bind_rustls(bind_addr, rustls_config)
                    .handle(handle.clone())
                    .serve(app.into_make_service()),
            );
        }

        select! {
            _ = workers.join_next() => {
                graceful_shutdown(workers, handle).await;
            }
            _ = shutdown_rx.recv() => {
                graceful_shutdown(workers, handle).await;
            }
        }
        Ok(())
    }
}

async fn graceful_shutdown(mut workers: JoinSet<IOResult<()>>, handle: Handle) {
    info!("Shutting down servers");
    handle.graceful_shutdown(Some(Duration::from_secs(30)));
    workers.shutdown().await;
}
