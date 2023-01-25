use anyhow::{anyhow, Result};
use axum::async_trait;
use axum::body::StreamBody;
use axum::http::response::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use std::sync::Arc;
use tokio::io::{duplex, AsyncWriteExt};
use tokio::spawn;
use tokio_util::io::ReaderStream;

use crate::cache::{
    tlrfu::{ByteSize, Tlrfu},
    Cache, CacheClient,
};
use crate::core::event::ProxyEvent;
use tokio::sync::{mpsc::UnboundedSender, oneshot, oneshot::Sender, RwLock};
use tracing::{error, info, warn};

#[derive(Debug)]
pub enum TlrfuCacheCommand {
    GetSync { key: String },
    InsertSync { key: String, value: Arc<Bytes> },
    TtlCleanUp,
}

pub struct TlrfuCache {
    tlrfu: Tlrfu<Bytes>,
    tx: UnboundedSender<TlrfuCacheCommand>,
    stream_buf: u64,
    cache_control_max_size: u64,
}

impl TlrfuCache {
    pub fn new(
        max_size: u64,
        ttl_buf: u128,
        tx: UnboundedSender<TlrfuCacheCommand>,
        stream_buf: u64,
        cache_control_max_size: u64,
    ) -> Self {
        Self {
            tlrfu: Tlrfu::new(max_size, ttl_buf),
            tx,
            stream_buf,
            cache_control_max_size,
        }
    }
}

impl TlrfuCache {
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

impl ByteSize for Bytes {
    fn len(&self) -> usize {
        self.len()
    }
}

#[derive(Clone)]
pub struct TCache(Arc<RwLock<TlrfuCache>>);

#[async_trait]
impl Cache for TCache {
    type Command = TlrfuCacheCommand;

    async fn handle(&mut self, cmd: Self::Command) {
        let cache = self.0.clone();
        match cmd {
            TlrfuCacheCommand::GetSync { key } => {
                spawn(async move {
                    info!("Process GetSyncAnnounce command with key: {key:?}");
                    if let Err(e) = cache.write().await.get(&key).await {
                        error!("Process GetSyncAnnounce command error with key: {key:?} {e:?}");
                        // TODO: do we need this?
                        // signal_tx.send(()).await.expect("Send signal successfully");
                    };
                });
            }
            TlrfuCacheCommand::InsertSync { key, value } => {
                spawn(async move {
                    info!("Process InsertSyncAnnounce command with key: {key:?}");
                    if let Err(e) = cache.write().await.insert(String::from(&key), value).await {
                        error!("Process InsertSyncAnnounce command error with key: {key:?} {e:?}");
                    };
                });
            }
            TlrfuCacheCommand::TtlCleanUp => {
                spawn(async move {
                    info!("Process TtlCleanUp command");
                    if let Err(e) = cache.write().await.ttl_cleanup().await {
                        error!("Process TtlCleanUp command error {e:?}");
                    };
                });
            }
        }
    }
}

#[async_trait]
impl CacheClient for TCache {
    async fn query_cache(&self, k: &str, _: bool) -> Result<Option<Response>> {
        let cache = self.0.read().await;
        if let Some(data) = cache.tlrfu.dirty_get(&String::from(k)) {
            let (mut w, r) = duplex(cache.stream_buf as usize);
            let data = Arc::clone(data);
            cache
                .tx
                .send(TlrfuCacheCommand::GetSync {
                    key: String::from(k),
                })
                .map_err(|e| {
                    error!("Failed to dispatch GetSync command: {e:?}");
                    anyhow!("Failed to dispatch GetSync command")
                })?;
            spawn(async move {
                if let Err(e) = w.write_all(data.as_ref()).await {
                    warn!("Failed to write to stream: {e:?}");
                }
            });
            return Ok(Some(StreamBody::new(ReaderStream::new(r)).into_response()));
        }
        Ok(None)
    }

    async fn handle_proxy_event(&self, event: ProxyEvent) {
        match event {
            ProxyEvent::UpstreamData(_) => {}
            ProxyEvent::Error(_) => {}
        }
    }
}
