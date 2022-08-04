use metrics::{
    increment_gauge, decrement_gauge, increment_counter,
};
use tracing::info;

pub const PEER_CONNECTED: &str = "peer_connected";
pub const PEER_DISCONNECTED: &str = "peer_disconnected";
pub const BITSWAP: &str = "bitswap";
pub const GOSSIP_MESSAGE: &str = "gossip_message";
pub const REQUEST_MESSAGE: &str = "request_message";
pub const RPC_REQUEST_RECEIVED: &str = "rpc_request_received";


const ACTIVE_CONNECTED_PEERS: &str = "active_connected_peers";
const HTTP_RPC_REQUESTS: &str = "http_rpc_requests_total";
const NODE_BITSWAP_OPERATIONS: &str = "bitswap_operations_total";
const NODE_GOSSIP_MESSAGES: &str = "gossip_messages_total";
const NODE_REQUEST_MESSAGES: &str = "request_messages_total";



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
        
        BITSWAP => {
            increment_counter!(NODE_BITSWAP_OPERATIONS);
        }

        GOSSIP_MESSAGE => {
            increment_counter!(NODE_GOSSIP_MESSAGES);
        }

        REQUEST_MESSAGE => {
            increment_counter!(NODE_REQUEST_MESSAGES);
        }

        _ => info!("event name {:?} no match found", event_name)
    }
}
