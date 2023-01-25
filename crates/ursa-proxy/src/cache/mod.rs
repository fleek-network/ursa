pub mod moka_cache;
mod tlrfu;
mod tlrfu_cache;

use crate::core::event::ProxyEvent;
use anyhow::Result;
use axum::{async_trait, response::Response};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

/// Trait that handles caching commands from CacheClient.
#[async_trait]
pub trait Cache: Clone + Send + Sync + 'static {
    type Command: Send;
    async fn handle(&mut self, cmd: Self::Command);
}

/// Cache client trait.
///
/// Implement this trait to send commands to your Cache implementation.
#[async_trait]
pub trait CacheClient: Cache {
    async fn query_cache(&self, k: &str, no_cache: bool) -> Result<Option<Response>>;
    async fn handle_proxy_event(&self, event: ProxyEvent);
    /// This method is only called once.
    async fn command_receiver(&mut self) -> UnboundedReceiver<Self::Command>;
}

#[async_trait]
impl<T: Cache> Cache for Arc<T> {
    type Command = T::Command;

    async fn handle(&mut self, cmd: Self::Command) {
        self.handle(cmd).await;
    }
}

#[async_trait]
impl<T: CacheClient> CacheClient for Arc<T> {
    async fn query_cache(&self, k: &str, no_cache: bool) -> Result<Option<Response>> {
        self.query_cache(k, no_cache).await
    }

    async fn handle_proxy_event(&self, event: ProxyEvent) {
        self.handle_proxy_event(event).await;
    }

    async fn command_receiver(&mut self) -> UnboundedReceiver<Self::Command> {
        self.command_receiver().await
    }
}
