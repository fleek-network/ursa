use libp2p::PeerId;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackerRegistration {
    pub id: PeerId,
    pub agent: String,
    pub addr: Option<String>,
    pub p2p_port: Option<u16>,
    pub http_port: Option<u16>,
    pub telemetry: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: PeerId,
    pub agent: String,
    pub addr: String,
    pub p2p_port: u16,
    pub http_port: u16,
    pub telemetry: bool,
    pub geohash: String,
    pub timezone: String,
    pub country_code: String,
    pub last_registered: u64,
}

impl Node {
    pub fn from_info(
        registration: &TrackerRegistration,
        addr: String,
        geohash: String,
        timezone: String,
        country_code: String,
    ) -> Self {
        Self {
            id: registration.id,
            agent: registration.agent.clone(),
            addr: registration.addr.clone().unwrap_or(addr),
            p2p_port: registration.p2p_port.unwrap_or(6009),
            http_port: registration.http_port.unwrap_or(4069),
            telemetry: registration.telemetry.unwrap_or(true),
            geohash,
            timezone,
            country_code,
            last_registered: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
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
        labels.insert("agent".to_string(), node.agent.clone());
        PrometheusDiscoveryChunk::new(vec![format!("{}:{}", node.addr, node.http_port)], labels)
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
