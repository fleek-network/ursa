mod handler;

use crate::{
    cache::Cache,
    config::{ProxyConfig, ServerConfig},
    core::handler::{init_admin_app, init_server_app},
};
use anyhow::{Context, Result};
use axum_server::{tls_rustls::RustlsConfig, Handle};
use hyper::Client;
use std::{io::Result as IOResult, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{select, sync::mpsc::Receiver, task::JoinSet};
use tracing::info;

#[derive(Clone)]
pub struct Server {
    pub tls_config: Option<RustlsConfig>,
    pub config: Arc<ServerConfig>,
}

pub async fn start<C: Cache>(
    config: ProxyConfig,
    cache: C,
    mut shutdown_rx: Receiver<()>,
) -> Result<()> {
    let mut workers = JoinSet::new();
    let handle = Handle::new();
    let client = Client::new();
    let mut servers = Vec::new();
    for server_config in config.server {
        let server_config = Arc::new(server_config);
        let server_app = init_server_app(server_config.clone(), cache.clone(), client.clone());
        let bind_addr = server_config
            .listen_addr
            .clone()
            .parse::<SocketAddr>()
            .context("Invalid binding address")?;
        if let Some(server_tls_config) = server_config.tls.as_ref() {
            let tls_config = RustlsConfig::from_pem_file(
                &server_tls_config.cert_path,
                &server_tls_config.key_path,
            )
            .await?;
            servers.push(Server {
                tls_config: Some(tls_config.clone()),
                config: server_config.clone(),
            });
            workers.spawn(
                axum_server::bind_rustls(bind_addr, tls_config)
                    .handle(handle.clone())
                    .serve(server_app),
            );
        } else {
            workers.spawn(
                axum_server::bind(bind_addr)
                    .handle(handle.clone())
                    .serve(server_app),
            );
        }
        info!("Listening on {bind_addr:?}");
    }

    let admin_handle = handle.clone();
    let admin_addr = config.admin.unwrap_or_default().addr.parse()?;
    workers.spawn(async move {
        let admin_app = init_admin_app(cache.clone(), servers);
        axum_server::bind(admin_addr)
            .handle(admin_handle)
            .serve(admin_app.into_make_service())
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
