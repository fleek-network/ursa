use async_trait::async_trait;
use tiny_cid::Cid;

#[async_trait]
pub trait NetworkInterface<T>: Clone + Send + Sync + 'static {
    type Error;

    async fn put(&self, cid: Cid) -> Result<(), Self::Error>;

    async fn get(&self, cid: Cid) -> Result<(), Self::Error>;
}
