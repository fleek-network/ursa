use anyhow::{anyhow, Result};
use async_fs::File;
use async_trait::async_trait;
use axum::body::StreamBody;
use cid::Cid;
use db::Store as Store_;
use futures::channel::mpsc::unbounded;
use futures::io::BufReader;
use futures::{AsyncRead, AsyncWriteExt, SinkExt};
use fvm_ipld_blockstore::Blockstore;
use fvm_ipld_car::{load_car, CarHeader};
use serde::{Deserialize, Serialize};
use ursa_index_provider::engine::ProviderCommand;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::create_dir_all;
use tokio::sync::mpsc::UnboundedSender as Sender;
use tokio::sync::{oneshot, RwLock};
use tokio::task;
use tokio_util::{compat::TokioAsyncWriteCompatExt, io::ReaderStream};
use tracing::info;
use ursa_network::NetworkCommand;
use ursa_store::{Dag, Store};
use ursa_utils::convert_cid;

pub const MAX_BLOCK_SIZE: usize = 1048576;
pub const MAX_CHUNK_SIZE: usize = 104857600;
pub const DEFAULT_CHUNK_SIZE: usize = 10 * 1024 * 1024; // chunk to ~10MB CARs

/// Network Api
#[derive(Deserialize, Serialize)]
pub struct NetworkGetParams {
    pub cid: String,
}

pub type NetworkGetResult = Vec<u8>;
pub const NETWORK_GET: &str = "ursa_get_cid";

#[derive(Deserialize, Serialize)]
pub struct NetworkPutFileParams {
    pub path: String,
}

pub type NetworkPutFileResult = String;
pub const NETWORK_PUT_FILE: &str = "ursa_put_file";

#[derive(Deserialize, Serialize)]
pub struct NetworkGetFileParams {
    pub path: String,
    pub cid: String,
}
pub const NETWORK_GET_FILE: &str = "ursa_get_file";

/// Abstraction of Ursa's server commands
#[async_trait]
pub trait NetworkInterface: Sync + Send + 'static {
    /// Get a bitswap block from the network
    async fn get(&self, cid: Cid) -> Result<Option<Vec<u8>>>;

    async fn get_data(&self, root_cid: Cid) -> Result<Vec<(Cid, Vec<u8>)>>;

    /// get the file locally via cli
    async fn get_file(&self, path: String, cid: Cid) -> Result<()>;

    // stream the car file from server
    async fn stream(
        &self,
        root_cid: Cid,
    ) -> Result<StreamBody<ReaderStream<tokio::io::DuplexStream>>>;

    /// Put a car file and start providing to the network
    async fn put_car<R: AsyncRead + Send + Unpin>(&self, reader: R) -> Result<Vec<Cid>>;

    // Put a file using a local path
    async fn put_file(&self, path: String) -> Result<Vec<Cid>>;
}
#[derive(Clone)]
pub struct NodeNetworkInterface<S>
where
    S: Blockstore + Store_ + Send + Sync + 'static,
{
    pub store: Arc<Store<S>>,
    pub network_send: Sender<NetworkCommand>,
    pub provider_send: Sender<ProviderCommand>,
}

#[async_trait]
impl<S> NetworkInterface for NodeNetworkInterface<S>
where
    S: Blockstore + Store_ + Send + Sync + 'static,
{
    async fn get(&self, cid: Cid) -> Result<Option<Vec<u8>>> {
        if !self.store.blockstore().has(&cid)? {
            info!("Requesting block with the cid {cid:?}");
            let (sender, receiver) = oneshot::channel();
            let request = NetworkCommand::GetBitswap { cid, sender };

            // use network sender to send command
            self.network_send.send(request)?;
            if let Err(e) = receiver.await? {
                return Err(anyhow!(
                    "The bitswap failed, please check server logs {:?}",
                    e
                ));
            }
        }
        self.store.blockstore().get(&cid)
    }

    async fn get_data(&self, root_cid: Cid) -> Result<Vec<(Cid, Vec<u8>)>> {
        if !self.store.blockstore().has(&root_cid)? {
            let (sender, receiver) = oneshot::channel();
            let request = NetworkCommand::GetBitswap {
                cid: root_cid,
                sender,
            };

            // use network sender to send command
            self.network_send.send(request)?;
            if let Err(e) = receiver.await? {
                return Err(anyhow!(
                    "The bitswap failed, please check server logs {:?}",
                    e
                ));
            }
        }
        let dag = self
            .store
            .dag_traversal(&convert_cid(root_cid.to_bytes()))?;
        info!("Dag traversal done, now streaming the file");

        Ok(dag)
    }

    /// Used through CLI
    async fn get_file(&self, path: String, root_cid: Cid) -> Result<()> {
        info!("getting and storing the file at: {path}");

        let header = CarHeader {
            roots: vec![root_cid],
            version: 1,
        };

        let buffer: Arc<RwLock<Vec<u8>>> = Default::default();
        let (mut tx, mut rx) = unbounded();

        let buffer_cloned = buffer.clone();
        let write_task = tokio::task::spawn(async move {
            header
                .write_stream_async(&mut *buffer_cloned.write().await, &mut rx)
                .await
                .unwrap()
        });
        let dag = self.get_data(root_cid).await?;

        for (cid, data) in dag {
            tx.send((convert_cid(cid.to_bytes()), data)).await?;
        }
        drop(tx);
        write_task.await?;

        let buffer: Vec<_> = buffer.read().await.clone();
        let file_path = PathBuf::from(path).join(format!("{}.car", root_cid));
        create_dir_all(file_path.parent().unwrap()).await?;
        let mut file = File::create(file_path).await?;
        file.write_all(&buffer).await?;
        Ok(())
    }

    async fn stream(
        &self,
        root_cid: Cid,
    ) -> Result<StreamBody<ReaderStream<tokio::io::DuplexStream>>> {
        let header = CarHeader {
            roots: vec![root_cid],
            version: 1,
        };

        let (mut tx, mut rx) = unbounded();
        let (writer, reader) = tokio::io::duplex(1024 * 100);

        let body = axum::body::StreamBody::new(ReaderStream::new(reader));

        task::spawn(async move {
            header
                .write_stream_async(&mut writer.compat_write(), &mut rx)
                .await
                .unwrap()
        });
        let dag = self.get_data(root_cid).await?;

        for (cid, data) in dag {
            tx.send((convert_cid(cid.to_bytes()), data)).await?;
        }
        drop(tx);

        Ok(body)
    }

    async fn put_car<R: AsyncRead + Send + Unpin>(&self, reader: R) -> Result<Vec<Cid>> {
        let cids = load_car(self.store.blockstore(), reader).await?;
        let root_cid = cids[0];

        info!("The inserted cids are: {cids:?}");

        let (sender, receiver) = oneshot::channel();
        let request = ProviderCommand::Put {
            context_id: root_cid.to_bytes(),
            sender,
        };

        self.provider_send.send(request)?;
        match receiver.await {
            Ok(_) => Ok(cids),
            Err(e) => Err(anyhow!(format!(
                "The PUT failed, please check server logs {:?}",
                e
            ))),
        }
    }

    /// Used through CLI
    async fn put_file(&self, path: String) -> Result<Vec<Cid>> {
        info!("Putting the file on network: {path}");
        let file = File::open(path.clone()).await?;
        let reader = BufReader::new(file);
        self.put_car(reader).await
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use db::{rocks::RocksDb, rocks_config::RocksDbConfig};
    use libp2p::identity::Keypair;
    use simple_logger::SimpleLogger;
    use tokio::task;
    use tracing::{error, log::LevelFilter};
    use ursa_network::{NetworkConfig, UrsaService};
    use ursa_store::Store;

    fn setup_logger(level: LevelFilter) {
        SimpleLogger::new()
            .with_level(level)
            .with_utc_timestamps()
            .init()
            .unwrap()
    }

    fn get_store(path: &str) -> Arc<Store<RocksDb>> {
        let db = Arc::new(
            RocksDb::open(path, &RocksDbConfig::default()).expect("Opening RocksDB must succeed"),
        );
        Arc::new(Store::new(Arc::clone(&db)))
    }

    #[tokio::test]
    async fn test_stream() -> Result<()> {
        setup_logger(LevelFilter::Info);
        let config = NetworkConfig::default();
        let keypair = Keypair::generate_ed25519();

        let store = get_store("test_db1");
        let service = UrsaService::new(keypair, &config, Arc::clone(&store))?;
        let rpc_sender = service.command_sender().clone();

        // Start libp2p service
        task::spawn(async {
            if let Err(err) = service.start().await {
                error!("[service_task] - {:?}", err);
            }
        });

        let interface = Arc::new(NodeNetworkInterface {
            store,
            network_send: rpc_sender,
        });

        let cids = interface
            .put_file("../../car_files/text_b.car".to_string())
            .await?;
        interface.stream(cids[0]).await?;

        Ok(())
    }
}
