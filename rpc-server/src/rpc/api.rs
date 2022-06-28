use std::sync::Arc;

use async_std::fs::File;
use async_trait::async_trait;
use car_rs::CarReader;
use futures::channel::oneshot;
use ipld_blockstore::BlockStore;
use jsonrpc_v2::Error;
use libipld::Cid;
use network::UrsaCommand;
use serde::{Deserialize, Serialize};
use store::Store;

pub type Result<T> = anyhow::Result<T, Error>;

pub const MAX_BLOCK_SIZE: usize = 1048576;
pub const MAX_CHUNK_SIZE: usize = 104857600;
pub const DEFAULT_CHUNK_SIZE: usize = 10 * 1024 * 1024; // chunk to ~10MB CARs

#[async_trait]
pub trait NetworkInterface: Sync + Send + 'static {
    type Error;

    async fn get(&self, cid: Cid) -> Result<()>;

    async fn put_car(&self, cid: Cid, car_reader: CarReader<File>) -> Result<()>;
}

pub struct NodeNetworkInterface<S>
where
    S: BlockStore + Sync + Send + 'static,
{
    pub store: Arc<Store<S>>,
}

#[async_trait]
impl<S> NetworkInterface for NodeNetworkInterface<S>
where
    S: BlockStore + Sync + Send + 'static,
{
    type Error = Error;

    async fn get(&self, cid: Cid) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        let request = UrsaCommand::Get { cid, sender };

        // use network sender to send command

        receiver.await;
        Ok(())
    }

    async fn put_car(&self, cid: Cid, car_reader: CarReader<File>) -> Result<()> {
        Ok(())
    }
}

/// Network Api
#[derive(Deserialize)]
pub struct NetworkGetParams {
    pub cid: Cid,
}

pub type NetworkGetResult = bool;

#[derive(Deserialize)]
pub struct NetworkPutCarParams {
    pub cid: Cid,
    // pub car: File,
}

pub type NetworkPutCarResult = bool;
