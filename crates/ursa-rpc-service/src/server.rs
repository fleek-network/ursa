use anyhow::Result;
use axum::{Extension, Router};
use db::Store;
use fvm_ipld_blockstore::Blockstore;
use std::{net::SocketAddr, sync::Arc};

use crate::{
    api::NodeNetworkInterface,
    config::ServerConfig,
    http,
    rpc::{self, RpcServer},
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

    pub async fn start(
        &self,
        config: &ServerConfig,
        index_provider: Router,
        metrics: Option<Router>,
    ) -> Result<()> {
        info!(
            "Server (rpc, http{}) starting up",
            if metrics.is_some() { " + metrics" } else { "" }
        );

        let service = MultiplexService::new(self.http_app(index_provider, metrics), self.rpc_app());

        let http_address = SocketAddr::from(([0, 0, 0, 0], config.port));
        info!("listening on {}", http_address);
        axum::Server::bind(&http_address)
            .serve(tower::make::Shared::new(service))
            .await?;

        Ok(())
    }

    pub fn rpc_app(&self) -> Router {
        Router::new()
            .merge(rpc::routes::network::init())
            .layer(Extension(self.rpc_server.clone()))
    }

    pub fn http_app(&self, index_provider: Router, metrics: Option<Router>) -> Router {
        Router::new()
            .merge(http::routes::network::init::<S>())
            .merge(index_provider)
            .merge(metrics.unwrap_or_else(Router::new))
            .layer(Extension(self.interface.clone()))
    }
}
