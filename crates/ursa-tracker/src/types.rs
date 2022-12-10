use libp2p::PeerId;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerRegistration {
    pub id: PeerId,
    pub storage: u64, // in bytes
    pub agent: String,
    pub addr: Option<String>,
    pub p2p_port: Option<u16>,
    pub rpc_port: Option<u16>,
    pub metrics_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: PeerId,
    pub agent: String,
    pub addr: String,
    pub p2p_port: u16,
    pub rpc_port: u16,
    pub telemetry: bool,
    pub metrics_port: u16,
    pub geohash: String,
    pub timezone: String,
    pub country_code: String,
}

impl Node {
    pub fn from_info(
        announcement: &TrackerRegistration,
        ip: String,
        geohash: String,
        timezone: String,
        country_code: String,
    ) -> Self {
        Self {
            id: announcement.id,
            agent: announcement.agent.clone(),
            addr: announcement.addr.clone().unwrap_or(ip),
            p2p_port: announcement.p2p_port.unwrap_or(6009),
            rpc_port: announcement.rpc_port.unwrap_or(4069),
            telemetry: announcement.metrics_port.is_some(),
            metrics_port: announcement.metrics_port.unwrap_or(4070),
            geohash,
            timezone,
            country_code,
        }
    }
}

impl From<Node> for PrometheusDiscoveryChunk {
    fn from(node: Node) -> Self {
        let mut labels = HashMap::new();
        labels.insert("id".to_string(), node.id.to_string());
        labels.insert("geohash".to_string(), node.geohash.clone());
        labels.insert("country_code".to_string(), node.country_code.clone());
        labels.insert("timezone".to_string(), node.timezone.clone());
        PrometheusDiscoveryChunk::new(vec![format!("{}:{}", node.addr, node.metrics_port)], labels)
    }
}

/// Prometheus HTTP service discovery chunk.
/// Targets are expected to provide a `/metrics` endpoint
#[derive(Serialize, Deserialize, Debug)]
pub struct PrometheusDiscoveryChunk {
    targets: Vec<String>,
    labels: HashMap<String, String>,
}

impl PrometheusDiscoveryChunk {
    pub(crate) fn new(targets: Vec<String>, labels: HashMap<String, String>) -> Self {
        Self { targets, labels }
    }
}
