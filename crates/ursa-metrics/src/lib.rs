pub mod config;
mod gossipsub;
mod identify;
mod kad;
pub mod middleware;
mod ping;
mod relay;
mod request_response;
pub mod server;
mod swarm;
pub use server::BITSWAP_REGISTRY;

/// Recorder that can record Swarm and protocol events.
pub trait Recorder {
    /// Record the given event.
    fn record(&self);
}
