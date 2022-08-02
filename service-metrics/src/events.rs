use metrics::{
    increment_gauge, decrement_gauge, increment_counter,
};
use tracing::info;

pub const PEER_CONNECTED: &str = "peer_connected";
pub const PEER_DISCONNECTED: &str = "peer_disconnected";
pub const RPC_REQUEST_RECEIVED: &str = "rpc_request_received";


const ACTIVE_CONNECTED_PEERS: &str = "active_connected_peers";
const HTTP_RPC_REQUESTS: &str = "http_rpc_requests_total";



pub fn track(event_name: &str) {
    info!("capturing event {:?}", event_name);
    match event_name {
        PEER_CONNECTED => {
            increment_gauge!(ACTIVE_CONNECTED_PEERS, 1.0);
        }
        PEER_DISCONNECTED => {
            decrement_gauge!(ACTIVE_CONNECTED_PEERS, 1.0);
        }

        RPC_REQUEST_RECEIVED => {
            increment_counter!(HTTP_RPC_REQUESTS);
        }

        _ => info!("event name {:?} no match found", event_name)
    }
}
