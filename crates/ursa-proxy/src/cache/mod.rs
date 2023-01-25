mod moka_cache;
mod tlrfu;
mod tlrfu_cache;

use axum::{async_trait, response::Response};

/// Trait that handles caching commands from CacheClient.
#[async_trait]
trait Cache: Send + Sync + 'static {
    type Command;
    async fn handle(&mut self, cmd: Self::Command) -> Result<(), String>;
}

/// Cache client trait.
///
/// Implement this trait to send commands to your Cache implementation.
#[async_trait]
trait CacheClient: Send + Sync + 'static {
    async fn get_announce(&self, k: &str, no_cache: bool) -> Result<Response, String>;
}
