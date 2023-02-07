pub mod moka_cache;

use crate::core::event::ProxyEvent;
use axum::async_trait;
use std::fmt::Debug;

/// Cache trait.
#[async_trait]
pub trait Cache: Clone + Send + Sync + 'static {
    type Command: Debug + Send;
    /// Handle events passed from the proxy.
    async fn handle_proxy_event(&self, event: ProxyEvent);
}
