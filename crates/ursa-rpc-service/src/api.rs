use anyhow::{anyhow, Result};
use async_fs::{create_dir_all, File};
use async_trait::async_trait;
use axum::body::StreamBody;
use db::Store;
use futures::channel::mpsc::unbounded;
use futures::io::BufReader;
use futures::{AsyncRead, AsyncWriteExt, SinkExt};
use fvm_ipld_blockstore::Blockstore;
use fvm_ipld_car::{load_car, CarHeader, CarReader};
use libipld::Cid;
use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use std::collections::{
    hash_map::{Entry, HashMap},
    HashSet,
};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use surf::{http::Method, Client, RequestBuilder};
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedSender as Sender},
    oneshot, RwLock,
};
use tokio::task;
use tokio_util::{compat::TokioAsyncWriteCompatExt, io::ReaderStream};
use tracing::{debug, error, info};
use ursa_index_provider::engine::ProviderCommand;
use ursa_network::NetworkCommand;
use ursa_store::UrsaStore;

use crate::config::OriginConfig;

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
    async fn get(&self, cid: Cid) -> Result<Vec<u8>>;

    /// Get content under a cid
    async fn get_data(&self, root_cid: Cid) -> Result<Vec<(Cid, Vec<u8>)>>;

    /// get the file locally via cli
    async fn get_file(&self, path: String, cid: Cid) -> Result<()>;

    /// Stream the car file from server
    async fn stream(
        &self,
        root_cid: Cid,
    ) -> Result<StreamBody<ReaderStream<tokio::io::DuplexStream>>>;

    /// Put a car file and start providing to the network
    async fn put_car<R: AsyncRead + Send + Unpin>(&self, file: Car<R>) -> Result<Vec<Cid>>;

    /// Put a file using a local path
    async fn put_file(&self, path: String) -> Result<Vec<Cid>>;

    /// Get peers from the network
    async fn get_peers(&self) -> Result<HashSet<PeerId>>;

    /// Get the addresses that p2p node is listening on
    async fn get_listener_addresses(&self) -> Result<Vec<Multiaddr>>;
}

type PendingRequests = Arc<RwLock<HashMap<Cid, Vec<Sender<Result<u64>>>>>>;

#[derive(Clone)]
pub struct NodeNetworkInterface<S>
where
    S: Blockstore + Store + Send + Sync + 'static,
{
    pub store: Arc<UrsaStore<S>>,
    pub network_send: Sender<NetworkCommand>,
    pub provider_send: Sender<ProviderCommand>,
    pending_requests: PendingRequests,
    client: Arc<Client>,
    origin_config: OriginConfig,
}

#[async_trait]
impl<S> NetworkInterface for NodeNetworkInterface<S>
where
    S: Blockstore + Store + Send + Sync + 'static,
{
    async fn get(&self, cid: Cid) -> Result<Vec<u8>> {
        self.sync_content(cid).await?;
        let content =
            self.store.blockstore().get(&cid)?.ok_or_else(|| {
                anyhow!("content was fetched but could not be found in blockstore")
            })?;
        Ok(content)
    }

    async fn get_data(&self, root_cid: Cid) -> Result<Vec<(Cid, Vec<u8>)>> {
        self.sync_content(root_cid).await?;
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

    async fn put_car<R: AsyncRead + Send + Unpin>(&self, car: Car<R>) -> Result<Vec<Cid>> {
        let size = car.size;
        let cids = load_car(self.store.blockstore(), car).await?;
        let root_cid = cids[0];
        info!("The inserted cids are: {cids:?}");
        self.provide_cid(root_cid, size).await.map(|_| cids)
    }

    /// Used through CLI
    async fn put_file(&self, path: String) -> Result<Vec<Cid>> {
        info!("Putting the file on network: {path}");
        self.put_car(Car::from_file(path).await?).await
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

impl<S> NodeNetworkInterface<S>
where
    S: Blockstore + Store + Send + Sync + 'static,
{
    pub fn new(
        store: Arc<UrsaStore<S>>,
        network_send: Sender<NetworkCommand>,
        provider_send: Sender<ProviderCommand>,
        origin_config: OriginConfig,
    ) -> Self {
        Self {
            store,
            network_send,
            provider_send,
            origin_config,
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            client: Arc::new(Client::new()),
        }
    }

    /// Ensure a root cid is synced to the blockstore
    async fn sync_content(&self, cid: Cid) -> Result<()> {
        if !self.store.blockstore().has(&cid)? {
            info!("Requesting block with the cid {cid:?}");

            let size = match self.get_network(cid).await {
                Ok(_) => self.store.car_size(&cid)?,
                Err(e) => {
                    info!("Failed to get content from network: {}", e);
                    self.get_origin(cid).await?
                }
            };
            self.provide_cid(cid, size).await
        } else {
            Ok(())
        }
    }

    /// Fetch content from the network
    async fn get_network(&self, root_cid: Cid) -> Result<()> {
        info!("Fetching cid {root_cid} from network");
        let (send, recv) = oneshot::channel();
        self.network_send.send(NetworkCommand::GetBitswap {
            cid: root_cid,
            sender: send,
        })?;
        recv.await?
    }

    /// Fetch content from the origin.
    /// Returns the size of the car file received
    async fn get_origin(&self, root_cid: Cid) -> Result<u64> {
        info!("Fetching cid {root_cid} from origin (ipfs)");
        let pending = self.pending_requests.clone();
        let (tx, mut rx) = unbounded_channel();
        match self.pending_requests.write().await.entry(root_cid) {
            Entry::Occupied(mut e) => {
                // there is a concurrent request for this cid, just wait for the first one and return
                e.get_mut().push(tx);
                return rx
                    .recv()
                    .await
                    .ok_or_else(|| anyhow!("Failed to receive status from channel"))?;
            }
            Entry::Vacant(e) => {
                e.insert(vec![tx]);
            }
        }

        // we are the first concurrent request for this cid
        let client = self.client.clone();

        let https = self
            .origin_config
            .use_https
            .map(|v| if v { "https://" } else { "http://" })
            .unwrap_or("https://");

        let req = RequestBuilder::new(
            Method::Get,
            format!("{https}{}/ipfs/{root_cid}", self.origin_config.ipfs_gateway).parse()?,
        )
        .header("Accept", "application/vnd.ipld.car")
        .build();

        let store = self.store.db.clone();
        task::spawn(async move {
            // send the request
            let result: Result<u64, String> = async {
                let mut res = client.send(req).await.map_err(|e| {
                    format!("Error getting content for cid {root_cid} from origin: {e}")
                })?;

                let body = res.body_bytes().await.map_err(|e| {
                    format!("Error receiving content for cid {root_cid} from origin: {e}")
                })?;
                let len = body.len() as u64;

                let car = CarReader::new(body.as_slice()).await.map_err(|e| {
                    format!("Error reading car file for cid {root_cid} from origin: {e}")
                })?;

                if car.header.roots.contains(&root_cid) {
                    car.read_into(store.as_ref())
                        .await
                        .map_err(|e| format!("Error storing cid {root_cid} from origin: {e}"))?;
                    Ok(len)
                } else {
                    Err(format!(
                        "Error: cid {root_cid} not found in the origin response car file"
                    ))
                }
            }
            .await;

            // notify pending requests for cids
            let mut pending = pending.write().await;
            match result {
                Ok(len) => {
                    if let Some(senders) = pending.remove(&root_cid) {
                        for sender in senders {
                            if sender.send(Ok(len)).is_err() {
                                debug!("Failed to send origin status to channel");
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(e);
                    if let Some(senders) = pending.remove(&root_cid) {
                        for sender in senders {
                            if sender.send(Err(anyhow!(e.clone()))).is_err() {
                                debug!("Failed to send origin status to channel");
                            }
                        }
                    }
                }
            }
        });

        rx.recv()
            .await
            .ok_or_else(|| anyhow!("Failed to receive status from channel"))?
    }

    /// Trigger the network and provider to start providing the content id.
    /// If the size is not provided, it will be calculated from the blockstore
    async fn provide_cid(&self, cid: Cid, size: u64) -> Result<()> {
        // network content replication
        let (sender, receiver) = oneshot::channel();
        if let Err(e) = self.network_send.send(NetworkCommand::Put { cid, sender }) {
            error!("Failed to send network command: {}", e);
        } else {
            match receiver.await {
                Ok(res) => res?,
                Err(e) => error!("Error receiving network put status for {cid}: {e}"),
            }
        }

        // provider announcement
        let (sender, receiver) = oneshot::channel();
        if let Err(e) = self.provider_send.send(ProviderCommand::Put {
            context_id: cid.to_bytes(),
            size,
            sender,
        }) {
            error!("Failed to announce content with cid {cid}: {e}");
        } else {
            match receiver.await {
                Ok(r) => return r,
                Err(e) => error!("Error receiving provider put status for {cid}: {e}"),
            };
        }

        Ok(())
    }
}

pub struct Car<R> {
    pub size: u64,
    reader: R,
}

impl<R> Car<R>
where
    R: AsyncRead + Send + Unpin,
{
    pub fn new(size: u64, reader: R) -> Self {
        Self { size, reader }
    }
}

impl Car<BufReader<File>> {
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path).await?;
        let size = file.metadata().await?.len();
        let reader = BufReader::new(file);
        Ok(Self::new(size, reader))
    }
}

impl<R> AsyncRead for Car<R>
where
    R: AsyncRead + Send + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.reader).poll_read(cx, buf)
    }
}
