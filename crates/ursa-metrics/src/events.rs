use metrics::{decrement_gauge, histogram, increment_counter, increment_gauge, Label};
use metrics::{describe_counter, describe_gauge, describe_histogram};
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub enum MetricEvent {
    PeerConnected,
    PeerDisconnected,
    RelayReservationOpened,
    RelayReservationClosed,
    RelayCircuitOpened,
    RelayCircuitClosed,
    Bitswap,
    GossipMessage,
    RequestMessage,
    RpcRequestReceived,
    RpcResponseSent,
}

#[derive(Debug, Clone)]
pub enum Metric {
    ActiveConnectedPeers,
    ActiveRelayReservations,
    ActiveRelayCircuits,
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
            Metric::ActiveRelayReservations => write!(f, "active_relay_reservations"),
            Metric::ActiveRelayCircuits => write!(f, "active_relay_circuits"),
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
            "active_relay_reservations" => Ok(Metric::ActiveRelayReservations),
            "active_relay_circuits" => Ok(Metric::ActiveRelayCircuits),
            "http_rpc_requests" => Ok(Metric::HttpRpcRequests),
            "node_bitswap_operations" => Ok(Metric::NodeBitswapOperations),
            "node_gossip_messages" => Ok(Metric::NodeGossipMessages),
            "node_request_messages" => Ok(Metric::NodeRequestMessages),
            "node_response_info" => Ok(Metric::NodeResponseInfo),
            _ => Ok(Metric::Unknown(s.to_string())),
        }
    }
}

pub fn describe() {
    // describe metrics
    describe_gauge!(
        Metric::ActiveConnectedPeers.to_string(),
        "Total number of connected peers"
    );
    describe_gauge!(
        Metric::ActiveRelayReservations.to_string(),
        "Total number of relay reservations"
    );
    describe_gauge!(
        Metric::ActiveRelayCircuits.to_string(),
        "Total number of relay circuits"
    );
    describe_counter!(
        Metric::NodeBitswapOperations.to_string(),
        "Total number of bitswap operations"
    );
    describe_counter!(
        Metric::NodeGossipMessages.to_string(),
        "Total number of gossip messages"
    );
    describe_counter!(
        Metric::NodeRequestMessages.to_string(),
        "Total number of requests"
    );
    describe_histogram!(Metric::NodeResponseInfo.to_string(), "Response latency");
}

pub fn track(event: MetricEvent, labels: Option<Vec<Label>>, value: Option<f64>) {
    let label = labels.clone().unwrap_or_default();
    info!(
        "capturing event {:?} {}",
        event,
        labels
            .map(|l| format!("with labels {:?}", l))
            .unwrap_or_default()
    );
    match event {
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
            Some(latency) => {
                histogram!(Metric::NodeResponseInfo.to_string(), latency, label)
            }
            None => error!(
                "missing required value for {} event",
                Metric::NodeResponseInfo
            ),
        },
        MetricEvent::PeerConnected => {
            increment_gauge!(Metric::ActiveConnectedPeers.to_string(), 1.0, label);
        }
        MetricEvent::PeerDisconnected => {
            decrement_gauge!(Metric::ActiveConnectedPeers.to_string(), 1.0, label);
        }
        MetricEvent::RpcRequestReceived => {
            // increment_counter!(Metric::ActiveConnectedPeers.to_string());
        }
        MetricEvent::RelayReservationOpened => {
            increment_gauge!(Metric::ActiveRelayReservations.to_string(), 1.0);
        }
        MetricEvent::RelayReservationClosed => {
            decrement_gauge!(Metric::ActiveRelayReservations.to_string(), 1.0);
        }
        MetricEvent::RelayCircuitOpened => {
            increment_gauge!(Metric::ActiveRelayCircuits.to_string(), 1.0);
        }
        MetricEvent::RelayCircuitClosed => {
            decrement_gauge!(Metric::ActiveRelayCircuits.to_string(), 1.0);
        }
    }
}
