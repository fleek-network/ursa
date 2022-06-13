use anyhow::Error;
use async_trait::async_trait;
use network::service::UrsaService;
use rpc::NetworkInterface;
use serde::Serialize;
use tiny_cid::Cid;

#[async_trait]
pub trait NetworkInterface<T>: Clone + Send + Sync + 'static {
    type Error;

    async fn put(&self, cid: Cid) -> Result<(), Self::Error>;

    async fn get(&self, cid: Cid) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug, Serialize)]
pub struct Params<T: Serialize>(pub T);

#[derive(Clone, Debug)]
pub struct URL {
    pub port: String,
    pub domain: String,
}

impl URL {
    pub fn new(domain: &str, port: &str) -> URL {
        URL {
            port: port.to_owned(),
            domain: domain.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RpcClient<T: Serialize> {
    pub network: Arc<UrsaService>,
}

impl<T: Serialize> RpcClient<T> {
    pub fn new(network: UrsaService) -> RpcClient<T> {
        Self { network }
    }

    pub fn send(&self, url: &URL) -> serde_json::Value {
        todo!()
    }
}

#[async_trait]
impl<T> NetworkInterface<T> for RpcClient<T>
where
    T: Serialize + Clone + Send + Sync + 'static,
{
    async fn put(&self, cid: Cid) -> Result<(), Error> {
        todo!()
    }

    async fn get(&self, cid: Cid) -> Result<(), Error> {
        todo!()
    }
}
