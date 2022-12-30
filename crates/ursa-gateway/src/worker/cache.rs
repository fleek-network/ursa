use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use axum::{
    body::{Body, HttpBody, StreamBody},
    http::{response::Parts, StatusCode},
    response::Response,
};
use bytes::{BufMut, Bytes};
use tokio::{
    io::{duplex, AsyncWriteExt, DuplexStream},
    spawn,
    sync::{mpsc::UnboundedSender, oneshot},
};
use tokio_util::io::ReaderStream;
use tracing::{error, info, log::warn};

use crate::cache::{ByteSize, Tlrfu};
use crate::util::error::Error;

impl ByteSize for Bytes {
    fn len(&self) -> usize {
        self.len()
    }
}

pub struct Cache {
    tlrfu: Tlrfu<Bytes>,
    tx: UnboundedSender<WorkerCacheCommand>,
    stream_buf: u64,
}

impl Cache {
    pub fn new(
        max_size: u64,
        ttl_buf: u128,
        tx: UnboundedSender<WorkerCacheCommand>,
        stream_buf: u64,
    ) -> Self {
        Self {
            tlrfu: Tlrfu::new(max_size, ttl_buf),
            tx,
            stream_buf,
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
        value: Arc<Bytes>,
    },
    Fetch {
        cid: String,
        sender: oneshot::Sender<std::result::Result<Response<Body>, Error>>,
    },
    TtlCleanUp,
}

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

#[async_trait]
pub trait ServerCache: Send + Sync + 'static {
    async fn get_announce(
        &self,
        k: &str,
    ) -> std::result::Result<StreamBody<ReaderStream<DuplexStream>>, Error>;
}

#[async_trait]
impl ServerCache for Cache {
    async fn get_announce(
        &self,
        k: &str,
    ) -> std::result::Result<StreamBody<ReaderStream<DuplexStream>>, Error> {
        let (mut w, r) = duplex(self.stream_buf as usize);
        if let Some(data) = self.tlrfu.dirty_get(&String::from(k)) {
            let data = Arc::clone(data);
            self.tx
                .send(WorkerCacheCommand::GetSync {
                    key: String::from(k),
                })
                .map_err(|e| {
                    error!("Failed to dispatch GetSync command: {e:?}");
                    anyhow!("Failed to dispatch GetSync command")
                })?;
            spawn(async move {
                if let Err(e) = w.write_all(data.as_ref()).await {
                    error!("Failed to write to stream: {e:?}");
                }
            });
        } else {
            let (tx, rx) = oneshot::channel();
            self.tx
                .send(WorkerCacheCommand::Fetch {
                    cid: String::from(k),
                    sender: tx,
                })
                .map_err(|e| {
                    error!("Failed to dispatch Fetch command: {e:?}");
                    anyhow!("Failed to dispatch Fetch command")
                })?;
            let mut body = match rx
                .await
                .map_err(|e| {
                    error!("Failed to receive receive response from resolver: {e:?}");
                    anyhow!("Failed to receive receive response from resolver")
                })??
                .into_parts()
            {
                (
                    Parts {
                        status: StatusCode::OK,
                        ..
                    },
                    body,
                ) => body,
                (parts, body) => {
                    error!("Error requested provider with parts :{parts:?} and body: {body:?}");
                    return Err(Error::Upstream(
                        parts.status,
                        "Error requested provider".to_string(),
                    ));
                }
            };
            let key = String::from(k); // move to [worker|writer] thread
            let tx = self.tx.clone(); // move to [worker|writer] thread
            spawn(async move {
                let mut bytes = Vec::with_capacity(body.size_hint().lower() as usize);
                while let Some(buf) = body.data().await {
                    match buf {
                        Ok(buf) => {
                            if let Err(e) = w.write_all(buf.as_ref()).await {
                                error!("Failed to write to stream for {e:?}");
                                return;
                            };
                            bytes.put(buf);
                        }
                        Err(e) => {
                            error!("Failed to read stream for {e:?}");
                            return;
                        }
                    }
                }
                if let Err(e) = tx.send(WorkerCacheCommand::InsertSync {
                    key,
                    value: Arc::new(bytes.into()),
                }) {
                    error!("Failed to dispatch InsertSync command: {e:?}");
                };
            });
        }
        Ok(StreamBody::new(ReaderStream::new(r)))
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
