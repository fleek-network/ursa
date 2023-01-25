pub mod moka_cache;
mod tlrfu;
mod tlrfu_cache;

use crate::core::event::ProxyEvent;
use anyhow::Result;
use axum::{async_trait, response::Response};
use std::sync::Arc;

/// Trait that handles caching commands from CacheClient.
#[async_trait]
pub trait Cache: Clone + Send + Sync + 'static {
    type Command;
    async fn handle(&mut self, cmd: Self::Command);
}

/// Cache client trait.
///
/// Implement this trait to send commands to your Cache implementation.
#[async_trait]
pub trait CacheClient: Clone + Send + Sync + 'static {
    async fn query_cache(&self, k: &str, no_cache: bool) -> Result<Option<Response>>;
    async fn handle_proxy_event(&self, event: ProxyEvent);
}

#[async_trait]
impl<T: CacheClient> CacheClient for Arc<T> {
    async fn query_cache(&self, k: &str, no_cache: bool) -> Result<Option<Response>> {
        self.query_cache(k, no_cache).await
    }

    async fn handle_proxy_event(&self, event: ProxyEvent) {
        self.handle_proxy_event(event).await;
    }
}
