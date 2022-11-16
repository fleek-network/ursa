use std::fmt::{Display, Formatter};
use std::str::FromStr;
use metrics::{decrement_gauge, histogram, increment_counter, increment_gauge, Label};
use tracing::{error, info};

#[derive(Debug, Clone)]
pub enum MetricEvent {
    PeerConnected,
    PeerDisconnected,
    Bitswap,
    GossipMessage,
    RequestMessage,
    RpcRequestReceived,
    RpcResponseSent,
}

#[derive(Debug, Clone)]
pub enum Metric {
    ActiveConnectedPeers,
    HttpRpcRequests,
    NodeBitswapOperations,
    NodeGossipMessages,
    NodeRequestMessages,
    NodeResponseInfo,
    Unknown(String),
}

impl Display for Metric {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Metric::ActiveConnectedPeers => write!(f, "active_connected_peers"),
            Metric::HttpRpcRequests => write!(f, "http_rpc_requests"),
            Metric::NodeBitswapOperations => write!(f, "node_bitswap_operations"),
            Metric::NodeGossipMessages => write!(f, "node_gossip_messages"),
            Metric::NodeRequestMessages => write!(f, "node_request_messages"),
            Metric::NodeResponseInfo => write!(f, "node_response_info"),
            Metric::Unknown(s) => write!(f, "{}", s),
        }
    }
}

impl FromStr for Metric {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active_connected_peers" => Ok(Metric::ActiveConnectedPeers),
            "http_rpc_requests" => Ok(Metric::HttpRpcRequests),
            "node_bitswap_operations" => Ok(Metric::NodeBitswapOperations),
            "node_gossip_messages" => Ok(Metric::NodeGossipMessages),
            "node_request_messages" => Ok(Metric::NodeRequestMessages),
            "node_response_info" => Ok(Metric::NodeResponseInfo),
            _ => Ok(Metric::Unknown(s.to_string())),
        }
    }
}


pub fn track(event_name: MetricEvent, labels: Option<Vec<Label>>, value: Option<f64>) {
    if let Some(label) = labels {
        info!("capturing event {:?} with labels {:?}", event_name, label);
        match event_name {
            MetricEvent::Bitswap => {
                increment_counter!(Metric::NodeBitswapOperations.to_string(), label);
            }
            MetricEvent::GossipMessage => {
                increment_counter!(Metric::NodeGossipMessages.to_string(), label);
            }
            MetricEvent::RequestMessage => {
                increment_counter!(Metric::NodeRequestMessages.to_string(), label);
            }
            MetricEvent::RpcResponseSent => match value {
                Some(latency) => histogram!(Metric::NodeResponseInfo.to_string(), latency, label),
                None => error!("missing required value for {} event", Metric::NodeResponseInfo),
            },
            _ => error!("label on non-labeled event {:?}", event_name),
        }
    } else {
        info!("capturing event {:?}", event_name);
        match event_name {
            MetricEvent::PeerConnected => {
                increment_gauge!(Metric::ActiveConnectedPeers.to_string(), 1.0);
            }
            MetricEvent::PeerDisconnected => {
                decrement_gauge!(Metric::ActiveConnectedPeers.to_string(), 1.0);
            }
            MetricEvent::RpcRequestReceived => {
                increment_counter!(Metric::ActiveConnectedPeers.to_string());
            }
            _ => info!("missing label for {:?}", event_name),
        }
    }
}
