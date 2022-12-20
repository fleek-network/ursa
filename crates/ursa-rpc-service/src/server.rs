use anyhow::Result;
use axum::{Extension, Router};
use db::Store;
use fvm_ipld_blockstore::Blockstore;
use std::{net::SocketAddr, sync::Arc};

use crate::{
    api::NodeNetworkInterface,
    config::ServerConfig,
    http,
    rpc::{routes, RpcServer},
    service::MultiplexService,
};
use tracing::info;

pub struct Server<S>
where
    S: Blockstore + Store + Send + Sync + 'static,
{
    rpc_server: RpcServer,
    interface: Arc<NodeNetworkInterface<S>>,
}

impl<S> Server<S>
where
    S: Blockstore + Store + Send + Sync + 'static,
{
    pub fn new(interface: Arc<NodeNetworkInterface<S>>) -> Self {
        Self {
            rpc_server: RpcServer::new(Arc::clone(&interface)),
            interface: interface.clone(),
        }
    }

    pub async fn start(&self, config: &ServerConfig, metrics: Option<Router>) -> Result<()> {
        info!(
            "Server (rpc, http{}) starting up",
            if metrics.is_some() { " + metrics" } else { "" }
        );

        let rpc = Router::new()
            .merge(routes::network::init())
            .layer(Extension(self.rpc_server.clone()));

        let http = Router::new()
            .merge(http::routes::network::init::<S>())
            .merge(metrics.unwrap_or_else(Router::new))
            .layer(Extension(self.interface.clone()));

        let http_address = SocketAddr::from(([0, 0, 0, 0], config.port));

        let service = MultiplexService::new(http, rpc);

        info!("listening on {}", http_address);
        axum::Server::bind(&http_address)
            .serve(tower::make::Shared::new(service))
            .await?;

        Ok(())
    }
}
