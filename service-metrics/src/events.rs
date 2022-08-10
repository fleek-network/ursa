use metrics::{decrement_gauge, histogram, increment_counter, increment_gauge, Label};
use tracing::{error, info};

pub const PEER_CONNECTED: &str = "peer_connected";
pub const PEER_DISCONNECTED: &str = "peer_disconnected";
pub const BITSWAP: &str = "bitswap";
pub const GOSSIP_MESSAGE: &str = "gossip_message";
pub const REQUEST_MESSAGE: &str = "request_message";
pub const RPC_REQUEST_RECEIVED: &str = "rpc_request_received";
pub const RPC_RESPONSE_SENT: &str = "rpc_response_sent";

const ACTIVE_CONNECTED_PEERS: &str = "active_connected_peers";
const HTTP_RPC_REQUESTS: &str = "http_rpc_requests_total";
const NODE_BITSWAP_OPERATIONS: &str = "bitswap_operations_total";
const NODE_GOSSIP_MESSAGES: &str = "gossip_messages_total";
const NODE_REQUEST_MESSAGES: &str = "request_messages_total";
const NODE_RESPONSE_INFO: &str = "response_messages_info";

pub fn track(event_name: &str, labels: Option<Vec<Label>>, value: Option<f64>) {
    if let Some(label) = labels {
        info!("capturing event {:?} with labels {:?}", event_name, label);
        match event_name {
            BITSWAP => {
                increment_counter!(NODE_BITSWAP_OPERATIONS, label);
            }

            GOSSIP_MESSAGE => {
                increment_counter!(NODE_GOSSIP_MESSAGES, label);
            }

            REQUEST_MESSAGE => {
                increment_counter!(NODE_REQUEST_MESSAGES, label);
            }

            RPC_RESPONSE_SENT => match value {
                Some(latency) => histogram!(NODE_RESPONSE_INFO, latency, label),
                None => error!("mising required value for {} event", NODE_RESPONSE_INFO),
            },

            _ => info!("event name {:?} no match found", event_name),
        }
    } else {
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

            _ => info!("event name {:?} no match found", event_name),
        }
    }
}
