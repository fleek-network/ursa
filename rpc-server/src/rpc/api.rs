use async_trait::async_trait;
use jsonrpc_v2::Error;
use serde::Deserialize;
use tiny_cid::Cid;

pub type Result<T> = anyhow::Result<T, Error>;

#[async_trait]
pub trait NetworkInterface<T>: Clone + Send + Sync + 'static {
    type Error;

    async fn put(&self, cid: Cid) -> Result<()>;

    async fn get(&self, cid: Cid) -> Result<()>;
}

/// Network Api
#[derive(Deserialize)]
pub struct NetworkGetParams {
    pub cid: Cid,
}

pub type NetworkGetResult = bool;

#[derive(Deserialize)]
pub struct NetworkPutParams {
    pub cid: Cid,
}

pub type NetworkPutResult = bool;
