use {
    crate::cache::Tlrfu,
    anyhow::Result,
    async_trait::async_trait,
    std::sync::Arc,
    tokio::sync::{mpsc::UnboundedSender, oneshot},
    tracing::log::warn,
};

pub struct Cache {
    tlrfu: Tlrfu,
    tx: UnboundedSender<WorkerCacheCommand>,
}

impl Cache {
    pub fn new(max_size: u64, ttl_buf: u128, tx: UnboundedSender<WorkerCacheCommand>) -> Self {
        Self {
            tlrfu: Tlrfu::new(max_size, ttl_buf),
            tx,
        }
    }
}

#[derive(Debug)]
pub enum WorkerCacheCommand {
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
    TtlCleanUp,
}

#[async_trait]
pub trait WorkerCache: Send + Sync + 'static {
    async fn get(&mut self, k: &str) -> Result<()>;
    async fn insert(&mut self, k: String, v: Arc<Vec<u8>>) -> Result<()>;
    async fn ttl_cleanup(&mut self) -> Result<()>;
}

#[async_trait]
impl WorkerCache for Cache {
    async fn get(&mut self, k: &str) -> Result<()> {
        self.tlrfu.get(&String::from(k)).await?;
        Ok(())
    }

    async fn insert(&mut self, k: String, v: Arc<Vec<u8>>) -> Result<()> {
        if !self.tlrfu.contains(&k) {
            self.tlrfu.insert(k, v).await?;
        } else {
            warn!("[Cache]: Attempt to insert existed key: {k}");
        }
        Ok(())
    }

    async fn ttl_cleanup(&mut self) -> Result<()> {
        self.tlrfu.process_ttl_clean_up().await?;
        Ok(())
    }
}

#[async_trait]
pub trait ServerCache: Send + Sync + 'static {
    async fn get_announce(&self, k: &str) -> Result<Arc<Vec<u8>>>;
}

#[async_trait]
impl ServerCache for Cache {
    async fn get_announce(&self, k: &str) -> Result<Arc<Vec<u8>>> {
        if let Some(data) = self.tlrfu.dirty_get(&String::from(k)) {
            self.tx.send(WorkerCacheCommand::GetSync {
                key: String::from(k),
            })?;
            Ok(Arc::clone(data))
        } else {
            let (tx, rx) = oneshot::channel();
            self.tx.send(WorkerCacheCommand::Fetch {
                cid: String::from(k),
                sender: tx,
            })?;
            let data = rx.await??;
            self.tx.send(WorkerCacheCommand::InsertSync {
                key: String::from(k),
                value: Arc::clone(&data),
            })?;
            Ok(data)
        }
    }
}

pub trait AdminCache: Send + Sync + 'static {
    fn purge(&mut self);
}

impl AdminCache for Cache {
    fn purge(&mut self) {
        self.tlrfu.purge();
    }
}
