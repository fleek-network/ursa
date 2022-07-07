use async_std::io::BufReader;
use std::sync::Arc;

use anyhow::Result;
use async_std::{channel::Sender, fs::File};
use async_trait::async_trait;
use car_rs::{load_car, CarReader};
use cid::Cid;
use futures::{channel::oneshot, AsyncRead};
use ipld_blockstore::BlockStore;
use jsonrpc_v2::Error;
use network::UrsaCommand;
use serde::{Deserialize, Serialize};
use store::Store;
use tracing::{error, info, warn};

pub const MAX_BLOCK_SIZE: usize = 1048576;
pub const MAX_CHUNK_SIZE: usize = 104857600;
pub const DEFAULT_CHUNK_SIZE: usize = 10 * 1024 * 1024; // chunk to ~10MB CARs

/// Network Api
#[derive(Deserialize, Serialize)]
pub struct NetworkGetParams {
    pub cid: Cid,
}

pub type NetworkGetResult = bool;

#[derive(Deserialize, Serialize)]
pub struct NetworkPutCarParams {
    pub cid: Cid,
    pub data: Vec<u8>,
}

pub type NetworkPutCarResult = bool;

#[derive(Deserialize, Serialize)]
pub struct NetworkPutFileParams {
    pub cid: Cid,
    pub path: String,
}

pub type NetworkPutFileResult = bool;

/// Abstraction of Ursa's rpc commands
#[async_trait]
pub trait NetworkInterface: Sync + Send + 'static {
    type Error;

    /// Get a bitswap block from the network
    async fn get(&self, cid: Cid) -> Result<Vec<u8>>;

    /// Put a car file and start providing to the network
    async fn put_car<R: AsyncRead + Send + Unpin>(&self, cid: Cid, reader: R) -> Result<()>;

    // Put a file using a local path
    async fn put_file(&self, cid: Cid, path: String) -> Result<()>;
}

pub struct NodeNetworkInterface<S>
where
    S: BlockStore + Sync + Send + 'static,
{
    pub store: Arc<Store<S>>,
    pub network_send: Sender<UrsaCommand>,
}

#[async_trait]
impl<S> NetworkInterface for NodeNetworkInterface<S>
where
    S: BlockStore + Sync + Send + 'static,
{
    type Error = Error;

    async fn get(&self, cid: Cid) -> Result<Vec<u8>> {
        let (sender, receiver) = oneshot::channel();
        let request = UrsaCommand::Get { cid, sender };

        // use network sender to send command
        self.network_send.send(request).await?;
        receiver.await?
    }

    async fn put_car<R: AsyncRead + Send + Unpin>(&self, cid: Cid, reader: R) -> Result<()> {
        let cids = load_car(self.store.blockstore(), reader).await?;

        let (sender, receiver) = oneshot::channel();
        let request = UrsaCommand::StartProviding { cid, sender };

        self.network_send.send(request).await?;
        receiver.await?;

        Ok(())
    }

    /// Used through CLI
    async fn put_file(&self, cid: Cid, path: String) -> Result<()> {
        let file = File::open(path).await?;
        let reader = BufReader::new(file);

        self.put_car(cid, reader).await
    }
}
