use crate::{cache::Cache, config::MokaConfig};
use axum::{
    async_trait,
    body::StreamBody,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use moka::sync::Cache as Moka;
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{duplex, AsyncWriteExt},
    task,
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
    pub fn new(config: MokaConfig) -> Self {
        Self {
            inner: Moka::builder()
                .max_capacity(config.max_capacity)
                .time_to_idle(Duration::from_millis(config.time_to_idle))
                .time_to_live(Duration::from_millis(config.time_to_live))
                .build(),
            stream_buf: config.stream_buf,
        }
    }
}

#[async_trait]
impl Cache for MokaCache {
    fn get(&self, key: String) -> Option<Response> {
        let mut response = None;
        if let Some(data) = self.inner.get(&key) {
            let (mut w, r) = duplex(self.stream_buf as usize);
            task::spawn(async move {
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
