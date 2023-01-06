use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use tracing::{info, log::warn};

use super::Cache;

#[async_trait]
pub trait WorkerCache: Send + Sync + 'static {
    async fn get(&mut self, k: &str) -> Result<()>;
    async fn insert(&mut self, k: String, v: Arc<Bytes>) -> Result<()>;
    async fn ttl_cleanup(&mut self) -> Result<()>;
}

#[async_trait]
impl WorkerCache for Cache {
    async fn get(&mut self, k: &str) -> Result<()> {
        self.tlrfu.get(&String::from(k)).await?;
        Ok(())
    }

    async fn insert(&mut self, k: String, v: Arc<Bytes>) -> Result<()> {
        if !self.tlrfu.contains(&k) {
            self.tlrfu.insert(k, v).await?;
        } else {
            warn!("[Cache]: Attempt to insert existed key: {k}");
        }
        Ok(())
    }

    async fn ttl_cleanup(&mut self) -> Result<()> {
        let count = self.tlrfu.process_ttl_clean_up().await?;
        info!("[Cache]: TTL cleanup total {count} record(s)");
        Ok(())
    }
}
