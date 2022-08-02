use anyhow::Result;
use axum::{Extension, Router};
use std::{net::SocketAddr, sync::Arc};
use service_metrics::service::MetricsService;

use crate::{
    config::RpcConfig,
    rpc::{
        api::NetworkInterface,
        routes,
        rpc::RpcServer,
    },
};
use tracing::info;

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
        info!("Rpc server starting up");
        let router = Router::new()
            .merge(routes::network::init())
            .layer(Extension(self.server.clone()));

        let http_address = SocketAddr::from(([0, 0, 0, 0], config.rpc_port));

        info!("listening on {}", http_address);
        axum::Server::bind(&http_address)
            .serve(router.into_make_service())
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use db::{rocks::RocksDb, rocks_config::RocksDbConfig};
    use libp2p::{identity::Keypair, PeerId};
    use simple_logger::SimpleLogger;
    use store::Store;
    use tracing::log::LevelFilter;

    use crate::rpc::api::NodeNetworkInterface;
    use network::{config::UrsaConfig, service::UrsaService};

    fn ursa_network_init(
        config: &UrsaConfig,
        store: Arc<Store<RocksDb>>,
    ) -> (UrsaService<RocksDb>, PeerId) {
        let keypair = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(keypair.public());

        let service = UrsaService::new(keypair, config, store);

        (service, local_peer_id)
    }

    #[tokio::test]
    async fn test_rpc_start() {
        SimpleLogger::new()
            .with_utc_timestamps()
            .with_level(LevelFilter::Info)
            .with_colors(true)
            .init()
            .unwrap();

        let config = RpcConfig {
            rpc_port: 4069,
            rpc_addr: "0.0.0.0".to_string(),
        };

        let db = RocksDb::open("test_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");
        let db = Arc::new(db);
        let store = Arc::new(Store::new(Arc::clone(&db)));

        let ursa_config = UrsaConfig::default();
        let (ursa_node, _) = ursa_network_init(&ursa_config, Arc::clone(&store));
        let ursa_node_sender = ursa_node.command_sender().clone();

        let interface = Arc::new(NodeNetworkInterface {
            store,
            network_send: ursa_node_sender,
        });

        let rpc = Rpc::new(&config, interface);

        let _ = rpc.start(config).await;
    }
}
