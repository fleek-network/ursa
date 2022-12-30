use anyhow::{anyhow, Result};
use async_fs::{create_dir_all, File};
use async_trait::async_trait;
use axum::body::StreamBody;
use cid::Cid;
use db::Store;
use futures::channel::mpsc::unbounded;
use futures::io::BufReader;
use futures::{AsyncRead, AsyncWriteExt, SinkExt};
use fvm_ipld_blockstore::Blockstore;
use fvm_ipld_car::{load_car, CarHeader};
use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender as Sender;
use tokio::sync::{oneshot, RwLock};
use tokio::task;
use tokio_util::{compat::TokioAsyncWriteCompatExt, io::ReaderStream};
use tracing::{error, info};
use ursa_index_provider::engine::ProviderCommand;
use ursa_network::NetworkCommand;
use ursa_store::{Dag, UrsaStore};

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

pub type NetworkGetPeers = HashSet<PeerId>;
pub const NETWORK_GET_PEERS: &str = "ursa_get_peers";

pub type NetworkGetListenerAddresses = Vec<Multiaddr>;
pub const NETWORK_LISTENER_ADDRESSES: &str = "ursa_listener_addresses";

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

    // get peers from the network
    async fn get_peers(&self) -> Result<HashSet<PeerId>>;

    // get the addrresses that p2p node is listening on
    async fn get_listener_addresses(&self) -> Result<Vec<Multiaddr>>;
}
#[derive(Clone)]
pub struct NodeNetworkInterface<S>
where
    S: Blockstore + Store + Send + Sync + 'static,
{
    pub store: Arc<UrsaStore<S>>,
    pub network_send: Sender<NetworkCommand>,
    pub provider_send: Sender<ProviderCommand>,
}

#[async_trait]
impl<S> NetworkInterface for NodeNetworkInterface<S>
where
    S: Blockstore + Store + Send + Sync + 'static,
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
        let dag = self.store.dag_traversal(&root_cid)?;
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
            tx.send((cid, data)).await?;
        }
        drop(tx);
        write_task.await?;

        let buffer: Vec<_> = buffer.read().await.clone();
        let file_path = PathBuf::from(path).join(format!("{root_cid}.car"));
        create_dir_all(file_path.parent().unwrap()).await?;
        let mut file = File::create(file_path).await?;
        file.write_all(&buffer).await?;
        file.sync_all().await?;
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

        let body = StreamBody::new(ReaderStream::new(reader));

        task::spawn(async move {
            if let Err(err) = header
                .write_stream_async(&mut writer.compat_write(), &mut rx)
                .await
            {
                error!("Error while streaming the car file {err:?}");
            }
        });
        let dag = self.get_data(root_cid).await?;

        for (cid, data) in dag {
            tx.send((cid, data)).await?;
        }
        drop(tx);

        Ok(body)
    }

    async fn put_car<R: AsyncRead + Send + Unpin>(&self, reader: R) -> Result<Vec<Cid>> {
        let cids = load_car(self.store.blockstore(), reader).await?;
        let root_cid = cids[0];

        info!("The inserted cids are: {cids:?}");

        let (sender, _) = oneshot::channel();
        let request = NetworkCommand::Put {
            cid: root_cid,
            sender,
        };
        if let Err(e) = self.network_send.send(request) {
            error!("There was an error while sending NetworkCommand::Put: {e}");
        }

        let (sender, receiver) = oneshot::channel();
        let request = ProviderCommand::Put {
            context_id: root_cid.to_bytes(),
            sender,
        };
        if let Err(e) = self.provider_send.send(request) {
            // this error can be ignored for test_put_and_get test case
            error!("there was an error while sending provider command {e}");
            return Ok(cids);
        }
        match receiver.await {
            Ok(_) => Ok(cids),
            Err(e) => Err(anyhow!(format!(
                "The PUT failed, please check server logs {e:?}"
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

    async fn get_peers(&self) -> Result<HashSet<PeerId>> {
        let (sender, receiver) = oneshot::channel();
        let request = NetworkCommand::GetPeers { sender };

        self.network_send.send(request)?;
        match receiver.await {
            Ok(peer) => Ok(peer),
            Err(e) => Err(anyhow!(format!("GetPeers NetworkCommand failed {e:?}"))),
        }
    }

    async fn get_listener_addresses(&self) -> Result<Vec<Multiaddr>> {
        let (sender, receiver) = oneshot::channel();
        let request = NetworkCommand::GetListenerAddresses { sender };

        self.network_send.send(request)?;
        match receiver.await {
            Ok(addresses) => Ok(addresses),
            Err(e) => Err(anyhow!(format!(
                "GetListenerAddresses NetworkCommand failed {e:?}"
            ))),
        }
    }
}
