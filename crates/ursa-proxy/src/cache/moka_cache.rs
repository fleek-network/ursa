use crate::cache::{Cache, CacheClient};
use crate::core::event::ProxyEvent;
use axum::{async_trait, response::Response};
use bytes::Bytes;
use moka::sync::Cache as Moka;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

#[derive(Clone)]
pub struct MokaCache(Moka<String, Arc<Bytes>>);

impl MokaCache {
    pub fn new() -> Self {
        Self(Moka::new(100_000))
    }
}

pub enum MokaCacheCmd {
    Get {
        key: String,
        sender: Sender<Result<Arc<Bytes>, String>>,
    },
    Insert {
        key: String,
        value: Arc<Bytes>,
    },
    Invalidate {
        key: String,
    },
}

#[async_trait]
impl Cache for MokaCache {
    type Command = MokaCacheCmd;

    async fn handle(&mut self, cmd: Self::Command) {
        match cmd {
            MokaCacheCmd::Get { key, sender } => {
                if let Some(value) = self.0.get(&key) {
                    // TODO: Handle error.
                    sender
                        .send(Ok(value))
                        .await
                        .map_err(|e| e.to_string())
                        .unwrap();
                }
            }
            MokaCacheCmd::Insert { key, value } => self.0.insert(key, value),
            MokaCacheCmd::Invalidate { key } => self.0.invalidate(&key),
        }
    }
}

#[async_trait]
impl CacheClient for MokaCache {
    async fn query_cache(&self, k: &str, no_cache: bool) -> Result<Response, String> {
        todo!()
    }

    async fn handle_proxy_event(&self, event: ProxyEvent) {
        todo!()
    }
}
