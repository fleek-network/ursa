use anyhow::Result;
use axum::{Extension, Router};
use serde::Serialize;
use std::{marker::PhantomData, net::SocketAddr, sync::Arc};

use crate::{
    config::RpcConfig,
    rpc::{api::NetworkInterface, routes, rpc::RpcServer},
};

pub struct Rpc<I, T>
where
    I: NetworkInterface<T>,
    T: Serialize + 'static,
{
    server: RpcServer,
    interface: Arc<I>,
    _marker: PhantomData<T>,
}

impl<I, T> Rpc<I, T>
where
    I: NetworkInterface<T>,
    T: Serialize + 'static,
{
    pub fn new(config: RpcConfig, interface: Arc<I>) -> Self {
        Self {
            server: RpcServer::new(&config, Arc::clone(&interface)),
            interface: interface.clone(),
            _marker: PhantomData,
        }
    }

    pub async fn start(&self, config: RpcConfig) -> Result<()> {
        let router = Router::new()
            .merge(routes::network::init())
            .layer(Extension(self.server.clone()));

        let http_address = SocketAddr::from(([127, 0, 0, 1], config.port));
        axum::Server::bind(&http_address)
            .serve(router.into_make_service())
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_rpc_start() {}
}
