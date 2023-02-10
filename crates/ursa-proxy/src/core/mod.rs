mod handler;

use crate::{
    cache::Cache,
    config::ProxyConfig,
    core::handler::{init_server_app, purge_cache_handler, reload_tls},
};
use anyhow::{Context, Result};
use axum::{routing::post, Extension, Router};
use axum_server::Handle;
use hyper::Client;
use std::collections::HashMap;
use std::{io::Result as IOResult, net::SocketAddr, time::Duration};
use tokio::{select, sync::mpsc::Receiver, task::JoinSet};
use tracing::info;

pub async fn start<C: Cache>(
    config: ProxyConfig,
    cache: C,
    mut shutdown_rx: Receiver<()>,
) -> Result<()> {
    let mut workers = JoinSet::new();
    let handle = Handle::new();
    let client = Client::new();
    let mut tls_configs = HashMap::new();
    for server_config in config.server {
        let server_app =
            init_server_app(server_config.clone(), cache.clone(), client.clone()).await;
        let server_name = server_config.listen_addr.clone();
        let bind_addr = server_config
            .listen_addr
            .clone()
            .parse::<SocketAddr>()
            .context("Invalid binding address")?;
        info!("Listening on {bind_addr:?}");
        let tls_config = server_app.tls_config.unwrap();
        tls_configs.insert(server_name, tls_config.clone());
        workers.spawn(
            axum_server::bind_rustls(bind_addr, tls_config.clone())
                .handle(handle.clone())
                .serve(server_app.app.into_make_service()),
        );
    }

    let admin_handle = handle.clone();
    let cache_clone = cache.clone();
    let admin_addr = config.admin.unwrap_or_default().addr.parse()?;
    workers.spawn(async move {
        let app = Router::new()
            .route("/purge", post(purge_cache_handler::<C>))
            .route("/reload-tls", post(reload_tls))
            .layer(Extension(cache_clone))
            .layer(Extension(tls_configs));
        axum_server::bind(admin_addr)
            .handle(admin_handle)
            .serve(app.into_make_service())
            .await
    });

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

async fn graceful_shutdown(mut workers: JoinSet<IOResult<()>>, handle: Handle) {
    info!("Shutting down servers");
    handle.graceful_shutdown(Some(Duration::from_secs(30)));
    workers.shutdown().await;
}
