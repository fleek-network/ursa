use anyhow::{Error, Result};
use async_trait::async_trait;

#[async_trait]
pub trait UrsaRpc<T>: Clone + Send + Sync + 'static {
    async fn put(&self) -> Result<(), Error>;

    async fn get(&self) -> Result<(), Error>;
}
