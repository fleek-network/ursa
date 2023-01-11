use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use axum::{Extension, Router};
use tracing::info;

use ursa_store::StoreBase;

use crate::{
    api::NodeNetworkInterface,
    config::ServerConfig,
    http,
    rpc::{routes, RpcServer},
    service::MultiplexService,
};

pub struct Server<S: StoreBase> {
    rpc_server: RpcServer,
    interface: Arc<NodeNetworkInterface<S>>,
}

impl<S: StoreBase> Server<S> {
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
            .merge(routes::network::init())
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
