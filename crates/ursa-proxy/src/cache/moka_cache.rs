use crate::cache::Cache;
use crate::core::event::ProxyEvent;
use anyhow::Result;
use axum::{async_trait, body::StreamBody, response::IntoResponse, response::Response};
use bytes::Bytes;
use moka::sync::Cache as Moka;
use std::sync::Arc;
use tokio::{
    io::{duplex, AsyncWriteExt},
    spawn,
};
use tokio_util::io::ReaderStream;
use tracing::warn;

#[derive(Clone)]
pub struct MokaCache {
    inner: Moka<String, Arc<Bytes>>,
    stream_buf: u64,
}

impl MokaCache {
    #[allow(unused)]
    pub fn new(stream_buf: u64) -> Self {
        Self {
            inner: Moka::new(100_000),
            stream_buf,
        }
    }
}

#[async_trait]
impl Cache for MokaCache {
    type Command = ();
    async fn query_cache(&self, k: &str, _: bool) -> Result<Option<Response>> {
        if let Some(data) = self.inner.get(&String::from(k)) {
            let (mut w, r) = duplex(self.stream_buf as usize);
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
        if let ProxyEvent::UpstreamData(key, data) = event {
            self.inner.insert(key, Arc::new(Bytes::from(data)));
        }
    }
}
