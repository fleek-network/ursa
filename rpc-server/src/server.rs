use anyhow::Result;
use axum::{Extension, Router};
use std::{net::SocketAddr, sync::Arc};

use crate::{
    config::RpcConfig,
    rpc::{api::NetworkInterface, routes, rpc::RpcServer},
};

pub struct Rpc<I>
where
    I: NetworkInterface,
{
    server: RpcServer,
    interface: Arc<I>,
}

impl<I> Rpc<I>
where
    I: NetworkInterface,
{
    pub fn new(config: &RpcConfig, interface: Arc<I>) -> Self {
        Self {
            server: RpcServer::new(&config, Arc::clone(&interface)),
            interface: interface.clone(),
        }
    }

    pub async fn start(&self, config: RpcConfig) -> Result<()> {
        let router = Router::new()
            .merge(routes::network::init())
            .layer(Extension(self.server.clone()));

        let http_address = SocketAddr::from(([127, 0, 0, 1], config.rpc_port));
        axum::Server::bind(&http_address)
            .serve(router.into_make_service())
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use db::rocks::RocksDb;
    use simple_logger::SimpleLogger;
    use store::Store;

    use crate::rpc::api::NodeNetworkInterface;

    #[tokio::test]
    async fn test_rpc_start() {
        SimpleLogger::new()
            .with_utc_timestamps()
            .with_colors(true)
            .init()
            .unwrap();

        let config = RpcConfig {
            rpc_port: 4069,
            rpc_addr: "0.0.0.0".to_string(),
        };

        let db = RocksDb::open("test_db").expect("Opening RocksDB must succeed");
        let db = Arc::new(db);
        let store = Arc::new(Store::new(Arc::clone(&db)));

        let interface = Arc::new(NodeNetworkInterface { store });

        let rpc = Rpc::new(&config, interface);

        let _ = rpc.start(config).await;
    }
}
