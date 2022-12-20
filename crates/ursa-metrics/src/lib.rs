use lazy_static::lazy_static;
use prometheus::Registry;
use std::sync::Arc;

mod gossipsub;
mod identify;
mod kad;
pub mod middleware;
mod ping;
mod relay;
mod request_response;
pub mod routes;
mod swarm;

lazy_static! {
    pub static ref BITSWAP_REGISTRY: Arc<Registry> = Arc::new(Registry::new());
}

/// Recorder that can record Swarm and protocol events.
pub trait Recorder {
    /// Record the given event.
    fn record(&self);
}
