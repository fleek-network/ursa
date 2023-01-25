mod moka_cache;
mod tlrfu;
mod tlrfu_cache;

use crate::core::event::ProxyEvent;
use axum::{async_trait, response::Response};

/// Trait that handles caching commands from CacheClient.
#[async_trait]
pub trait Cache: Send + Sync + 'static {
    type Command;
    async fn handle(&mut self, cmd: Self::Command);
}

/// Cache client trait.
///
/// Implement this trait to send commands to your Cache implementation.
#[async_trait]
pub trait CacheClient: Send + Sync + 'static {
    async fn query_cache(&self, k: &str, no_cache: bool) -> Result<Response, String>;
    async fn handle_proxy_event(&self, event: ProxyEvent);
}
