mod lru;
mod tlrfu;

use std::sync::Arc;

use anyhow::Result;
use tlrfu::Tlrfu;
use tokio::sync::{mpsc::UnboundedSender, oneshot};
use tracing::warn;

pub struct Cache {
    tlrfu: Tlrfu,
    tx: UnboundedSender<CacheCmd>,
}

#[derive(Debug)]
pub enum CacheCmd {
    GetSync {
        key: String,
    },
    InsertSync {
        key: String,
        value: Arc<Vec<u8>>,
    },
    Fetch {
        cid: String,
        sender: oneshot::Sender<Result<Arc<Vec<u8>>>>,
    },
}

impl Cache {
    pub fn new(max_size: u64, ttl_buf: u128, tx: UnboundedSender<CacheCmd>) -> Self {
        Self {
            tlrfu: Tlrfu::new(max_size, ttl_buf),
            tx,
        }
    }

    pub async fn get(&mut self, k: &String) -> Result<()> {
        self.tlrfu.get(k).await?;
        Ok(())
    }

    pub async fn insert(&mut self, k: String, v: Arc<Vec<u8>>) -> Result<()> {
        if !self.tlrfu.contains(&k) {
            self.tlrfu.insert(k, v).await?;
        } else {
            warn!("[Cache]: attempt to insert existed key: {k}");
        }
        Ok(())
    }

    pub fn purge(&mut self) {
        self.tlrfu.purge();
    }

    pub async fn get_announce(&self, k: &String) -> Result<Arc<Vec<u8>>> {
        if let Some(data) = self.tlrfu.dirty_get(k) {
            self.tx.send(CacheCmd::GetSync {
                key: String::from(k),
            })?;
            Ok(Arc::clone(data))
        } else {
            let (tx, rx) = oneshot::channel();
            self.tx.send(CacheCmd::Fetch {
                cid: String::from(k),
                sender: tx,
            })?;
            let data = rx.await??;
            self.tx.send(CacheCmd::InsertSync {
                key: String::from(k),
                value: Arc::clone(&data),
            })?;
            Ok(data)
        }
    }
}
