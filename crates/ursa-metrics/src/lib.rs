pub mod config;
mod identify;
mod kad;
pub mod server;
pub mod middleware;
mod ping;
mod swarm;
mod relay;
mod gossipsub;
mod request_response;

/// Recorder that can record Swarm and protocol events.
pub trait Recorder {
    /// Record the given event.
    fn record(&self);
}
