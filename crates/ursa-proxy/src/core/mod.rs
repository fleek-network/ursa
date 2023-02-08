mod handler;

use crate::{cache::Cache, config::ProxyConfig, core::handler::proxy_pass};
use anyhow::{Context, Result};
use axum::{
    http::StatusCode,
    routing::{get, post},
    Extension, Router,
};
use axum_server::{tls_rustls::RustlsConfig, Handle};
use hyper::Client;
use std::{io::Result as IOResult, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{select, sync::mpsc::Receiver, task::JoinSet};
use tracing::info;

pub struct Proxy<C> {
    config: ProxyConfig,
    cache: C,
}

impl<C: Cache> Proxy<C> {
    pub fn new(config: ProxyConfig, cache: C) -> Self {
        Self { config, cache }
    }

    pub async fn start(self, mut shutdown_rx: Receiver<()>) -> Result<()> {
        let mut workers = JoinSet::new();
        let handle = Handle::new();
        let admin_handle = handle.clone();
        let cache = self.cache.clone();
        let admin_addr = self.config.admin.unwrap_or_default().addr.parse()?;
        workers.spawn(async move {
            let app = Router::new()
                .route("/purge", post(purge_cache_handler::<C>))
                .layer(Extension(cache));
            axum_server::bind(admin_addr)
                .handle(admin_handle)
                .serve(app.into_make_service())
                .await
        });

        let client = Client::new();
        for server_config in self.config.server {
            let server_config = Arc::new(server_config);
            let app = Router::new()
                .route("/*path", get(proxy_pass::<C>))
                .layer(Extension(self.cache.clone()))
                .layer(Extension(client.clone()))
                .layer(Extension(server_config.clone()));
            let bind_addr = server_config
                .listen_addr
                .clone()
                .parse::<SocketAddr>()
                .context("Invalid binding address")?;
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

pub async fn purge_cache_handler<C: Cache>(Extension(cache): Extension<C>) -> StatusCode {
    cache.purge();
    StatusCode::OK
}
