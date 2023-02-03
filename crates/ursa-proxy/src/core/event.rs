use axum::response::Response;
use tokio::sync::oneshot::Sender;

/// Events from proxy.
pub enum ProxyEvent {
    /// Proxy is requesting data from the Cache.
    GetRequest {
        key: String,
        sender: Sender<Option<Response>>,
    },
    /// Proxy is informing the Cache that data from origin has been received.
    UpstreamData { key: String, value: Vec<u8> },
    /// Proxy is informing the Cache about timer event.
    Timer,
    /// Proxy is informing the Cache about a failure while handling a request.
    Error(String),
    /// Proxy is requesting cache to purge.
    Purge,
}
