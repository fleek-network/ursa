use anyhow::{anyhow, Result};
use async_trait::async_trait;
use cid::Cid;
use fnv::FnvHashSet;
use fvm_ipld_car::{load_car, Block};
use ipld_blockstore::BlockStore;
use libipld::{
    prelude::{Codec, References},
    store::StoreParams,
    Cid as ipldCid, DefaultParams, Ipld,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncRead, BufReader};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::oneshot;
use tracing::info;
use ursa_network::utils::convert_cid;
use ursa_network::{BitswapType, UrsaCommand};
use ursa_store::Store;

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
pub struct NetworkPutCarParams {
    pub cid: String,
    pub data: Vec<u8>,
}

pub type NetworkPutCarResult = String;
pub const NETWORK_PUT_CAR: &str = "ursa_put_car";

#[derive(Deserialize, Serialize)]
pub struct NetworkPutFileParams {
    pub path: String,
}

pub type NetworkPutFileResult = String;
pub const NETWORK_PUT_FILE: &str = "ursa_put_file";

/// Abstraction of Ursa's server commands
#[async_trait]
pub trait NetworkInterface: Sync + Send + 'static {
    /// Get a bitswap block from the network
    async fn get(&self, cid: Cid) -> Result<Option<Vec<u8>>>;

    // stream the car file from server
    async fn stream(&self, root_cid: Cid) -> Result<Vec<Vec<u8>>>;

    /// Put a car file and start providing to the network
    async fn put_car<R: AsyncRead + Send + Unpin>(&self, reader: R) -> Result<Vec<Cid>>;

    // Put a file using a local path
    async fn put_file(&self, path: String) -> Result<Vec<Cid>>;
}
#[derive(Clone)]
pub struct NodeNetworkInterface<S>
where
    S: BlockStore + Sync + Send + 'static,
{
    pub store: Arc<Store<S>>,
    pub network_send: UnboundedSender<UrsaCommand>,
}

#[async_trait]
impl<S> NetworkInterface for NodeNetworkInterface<S>
where
    S: BlockStore + Sync + Send + 'static,
{
    async fn get(&self, cid: Cid) -> Result<Option<Vec<u8>>> {
        if !self.store.blockstore().has(&cid).unwrap() {
            info!("Requesting block with the cid {cid:?}");
            let (sender, receiver) = oneshot::channel();
            let request = UrsaCommand::GetBitswap {
                cid,
                query: BitswapType::Get,
                sender,
            };

            // use network sender to send command
            self.network_send.send(request);
            if let Err(e) = receiver.await? {
                return Err(anyhow!(format!(
                    "The bitswap failed, please check server logs {:?}",
                    e
                )));
            }
        }
        self.store.blockstore().get(&cid)
    }

    async fn stream(&self, root_cid: Cid) -> Result<Vec<Vec<u8>>> {
        if !self.store.blockstore().has(&root_cid).unwrap() {
            let (sender, receiver) = oneshot::channel();
            let request = UrsaCommand::GetBitswap {
                cid: root_cid,
                query: BitswapType::Sync,
                sender,
            };

            // use network sender to send command
            self.network_send.send(request);
            if let Err(e) = receiver.await? {
                return Err(anyhow!(format!(
                    "The bitswap failed, please check server logs {:?}",
                    e
                )));
            }
        }
        let mut res = Vec::new();
        // get full dag starting with root id
        let mut current = FnvHashSet::default();
        let mut refs = FnvHashSet::default();
        current.insert(convert_cid::<ipldCid>(root_cid.to_bytes()));

        while let Some(cid) = current.iter().next().copied() {
            current.remove(&cid);
            if refs.contains(&cid) {
                continue;
            }
            match self.store.blockstore().get(&convert_cid(cid.to_bytes()))? {
                Some(data) => {
                    res.push(data.clone());
                    let next_block = Block {
                        cid: convert_cid(cid.to_bytes()),
                        data,
                    };
                    let _action = next_block.references(&mut current)?;
                    refs.insert(cid);
                }
                None => todo!(),
            }
        }
        Ok(res)
    }

    async fn put_car<R: AsyncRead + Send + Unpin>(&self, reader: R) -> Result<Vec<Cid>> {
        let cids = load_car(self.store.blockstore(), reader).await?;

        info!("The inserted cids are: {cids:?}");

        let (sender, receiver) = oneshot::channel();
        let request = UrsaCommand::StartProviding { cids, sender };

        self.network_send.send(request);
        receiver.await?
    }

    /// Used through CLI
    async fn put_file(&self, path: String) -> Result<Vec<Cid>> {
        info!("Putting the file on network: {path}");
        let file = File::open(path.clone()).await?;
        let reader = BufReader::new(file);
        let cids = load_car(self.store.blockstore(), reader).await?;
        Ok(cids)
    }
}

pub trait BlockLinks {
    type Params: StoreParams;
    fn references<E: Extend<ipldCid>>(&self, set: &mut E) -> Result<()>;
}

impl BlockLinks for Block {
    type Params = DefaultParams;

    /// Returns the references.
    fn references<E: Extend<ipldCid>>(&self, set: &mut E) -> Result<()>
    where
        Ipld: References<<DefaultParams as StoreParams>::Codecs>,
    {
        <DefaultParams as StoreParams>::Codecs::try_from(self.cid.codec())?
            .references::<Ipld, E>(&self.data, set)
    }
}
