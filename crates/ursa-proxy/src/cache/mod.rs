pub mod moka_cache;

use crate::core::event::ProxyEvent;
use axum::async_trait;
use std::fmt::Debug;
use tokio::sync::mpsc::UnboundedReceiver;

/// Cache trait.
#[async_trait]
pub trait Cache: Clone + Send + Sync + 'static {
    type Command: Debug + Send;
    /// Handle events passed from the proxy.
    async fn handle_proxy_event(&self, event: ProxyEvent);
    /// Returns receiver for sending commands to CacheWorker.
    ///
    /// This method is only called once. See [`CacheWorker`].
    async fn command_receiver(&mut self) -> Option<UnboundedReceiver<Self::Command>> {
        None
    }
}

/// Trait that handles caching commands from Cache.
///
/// Proxy will spawn a separate worker/task to poll this.
#[async_trait]
pub trait CacheWorker: Clone + Send + Sync + 'static {
    type Command: Debug + Send;
    async fn handle_command(&mut self, cmd: Self::Command);
}
