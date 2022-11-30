pub mod config;
pub mod events;
mod identify;
mod kad;
pub mod metrics;
pub mod middleware;
mod ping;
mod swarm;

/// Recorder that can record Swarm and protocol events.
pub trait Recorder {
    /// Record the given event.
    fn record(&self);
}
