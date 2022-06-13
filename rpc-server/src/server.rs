use anyhow::Result;
use axum::{routing::post, Extension, Router};
use serde::Serialize;
use std::{
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use crate::{
    config::RpcConfig,
    rpc::{api::NetworkInterface, rpc::RpcServer},
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
            .route("/rpc/v0", post(self.server.handler))
            .layer(Extension(self.server));

        let http_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), config.port);
        axum::Server::bind(&http_address)
            .serve(router.into_make_service())
            .await?;

        Ok(())
    }
}
