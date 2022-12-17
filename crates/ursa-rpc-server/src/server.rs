use anyhow::Result;
use axum::{Extension, Router};
use db::Store;
use fvm_ipld_blockstore::Blockstore;
use std::{net::SocketAddr, sync::Arc};

use crate::{
    api::NodeNetworkInterface,
    config::ServerConfig,
    http,
    rpc::{self, rpc::RpcServer},
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

    pub async fn start(&self, config: ServerConfig) -> Result<()> {
        info!("Server (Rpc and http) starting up");
        let rpc_router = Router::new()
            .merge(rpc::routes::network::init())
            .layer(Extension(self.rpc_server.clone()));

        let http = Router::new()
            .merge(http::routes::network::init::<S>())
            .layer(Extension(self.interface.clone()));

        let http_address = SocketAddr::from(([0, 0, 0, 0], config.port));

        let service = MultiplexService::new(http, rpc_router);

        info!("listening on {}", http_address);
        axum::Server::bind(&http_address)
            .serve(tower::make::Shared::new(service))
            .await?;

        Ok(())
    }
}
