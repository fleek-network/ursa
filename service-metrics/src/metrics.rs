// constants
pub const ACTIVE_CONNECTED_PEERS: &str = "active_connected_peers";
pub const PEER_CONNECTED: &str = "peer_connected";
pub const PEER_DISCONNECTED: &str = "peer_disconnected";

// Event defines an event to track into our metrics struct
pub trait MetricsRecorder {
    fn record(&self, event_name: &str);
}
