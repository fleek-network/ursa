pub mod moka_cache;
mod tlrfu;
mod tlrfu_cache;

use crate::core::event::ProxyEvent;
use anyhow::Result;
use axum::{async_trait, response::Response};
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

/// Trait that handles caching commands from Cache.
///
/// Proxy will spawn a separate worker/task to poll this.
#[async_trait]
pub trait CacheWorker: Clone + Send + Sync + 'static {
    type Command: Debug + Send;
    async fn handle(&mut self, cmd: Self::Command);
}

/// Cache trait.
///
/// Implement this trait to send commands to your CacheWorker.
#[async_trait]
pub trait Cache: Clone + Send + Sync + 'static {
    type Command: Debug + Send;
    /// Query cache for a value.
    ///
    /// Users can delegate query tasks to workers. See [`CacheWorker`].
    async fn query_cache(&self, k: &str, no_cache: bool) -> Result<Option<Response>>;
    /// Handle events passed from the proxy.
    async fn handle_proxy_event(&self, event: ProxyEvent);
    /// Returns receiver for sending commands to CacheWorker.
    ///
    /// This method is only called once. See [`CacheWorker`].
    async fn command_receiver(&mut self) -> Option<UnboundedReceiver<Self::Command>> {
        None
    }
}
