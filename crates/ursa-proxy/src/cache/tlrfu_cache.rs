use crate::{
    cache::{
        tlrfu::{ByteSize, Tlrfu},
        Cache, CacheWorker,
    },
    core::event::ProxyEvent,
};
use anyhow::Result;
use axum::{async_trait, body::StreamBody, response::IntoResponse};
use bytes::Bytes;
use std::sync::Arc;
use tokio::{
    io::{duplex, AsyncWriteExt},
    spawn,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        RwLock,
    },
};
use tokio_util::io::ReaderStream;
use tracing::{error, info, warn};

#[derive(Debug)]
pub enum TlrfuCacheCommand {
    GetSync { key: String },
    InsertSync { key: String, value: Arc<Bytes> },
    TtlCleanUp,
}

pub struct InnerTlrfuCache {
    tlrfu: Tlrfu<Bytes>,
    rx: Option<UnboundedReceiver<TlrfuCacheCommand>>,
    tx: UnboundedSender<TlrfuCacheCommand>,
    stream_buf: u64,
}

impl InnerTlrfuCache {
    pub fn new(max_size: u64, ttl_buf: u128, stream_buf: u64) -> Self {
        let (tx, rx) = unbounded_channel();
        Self {
            tlrfu: Tlrfu::new(max_size, ttl_buf),
            tx,
            rx: Some(rx),
            stream_buf,
        }
    }
}

impl InnerTlrfuCache {
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
pub struct TlrfuCache(Arc<RwLock<InnerTlrfuCache>>);

impl TlrfuCache {
    pub fn new(max_size: u64, ttl_buf: u128, stream_buf: u64) -> Self {
        Self(Arc::new(RwLock::new(InnerTlrfuCache::new(
            max_size, ttl_buf, stream_buf,
        ))))
    }
}

#[async_trait]
impl CacheWorker for TlrfuCache {
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
impl Cache for TlrfuCache {
    type Command = TlrfuCacheCommand;

    async fn handle_proxy_event(&self, event: ProxyEvent) {
        match event {
            ProxyEvent::GetRequest { key, sender } => {
                let mut response = None;
                let cache = self.0.read().await;
                if let Some(data) = cache.tlrfu.dirty_get(&key) {
                    let (mut w, r) = duplex(cache.stream_buf as usize);
                    let data = Arc::clone(data);
                    match cache.tx.send(TlrfuCacheCommand::GetSync { key }) {
                        Ok(_) => {
                            spawn(async move {
                                if let Err(e) = w.write_all(data.as_ref()).await {
                                    warn!("Failed to write to stream: {e:?}");
                                }
                            });
                            response = Some(StreamBody::new(ReaderStream::new(r)).into_response());
                        }
                        Err(e) => {
                            error!("Failed to dispatch GetSync command: {e:?}");
                        }
                    }
                }
                if sender.send(response).is_err() {
                    error!("Failed to send response");
                }
            }
            ProxyEvent::UpstreamData { key, value } => {
                let cache = self.clone();
                spawn(async move {
                    if let Err(e) = cache.0.read().await.tx.send(TlrfuCacheCommand::InsertSync {
                        key,
                        value: Arc::new(value.into()),
                    }) {
                        error!("Failed to dispatch InsertSync command: {e:?}");
                    };
                });
            }
            ProxyEvent::Timer => {
                let cache = self.0.read().await;
                if cache.tx.send(TlrfuCacheCommand::TtlCleanUp).is_err() {
                    error!("Failed to dispatch TtlCleanUp command");
                }
            }
            _ => {}
        }
    }

    async fn command_receiver(&mut self) -> Option<UnboundedReceiver<Self::Command>> {
        self.0.write().await.rx.take()
    }
}
