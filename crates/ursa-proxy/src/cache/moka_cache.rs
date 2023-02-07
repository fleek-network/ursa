use crate::cache::Cache;
use axum::{
    async_trait,
    body::StreamBody,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use moka::sync::Cache as Moka;
use std::sync::Arc;
use tokio::{
    io::{duplex, AsyncWriteExt},
    spawn,
};
use tokio_util::io::ReaderStream;
use tracing::{info, warn};

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
    fn get(&self, key: String) -> Option<Response> {
        let mut response = None;
        if let Some(data) = self.inner.get(&key) {
            let (mut w, r) = duplex(self.stream_buf as usize);
            spawn(async move {
                if let Err(e) = w.write_all(data.as_ref()).await {
                    warn!("Failed to write to stream: {e:?}");
                }
            });
            response = Some(StreamBody::new(ReaderStream::new(r)).into_response());
        }
        response
    }

    fn insert(&self, key: String, value: Vec<u8>) {
        self.inner.insert(key, Arc::new(Bytes::from(value)))
    }

    fn purge(&self) {
        info!("Invalidating data");
        self.inner.invalidate_all()
    }
}
