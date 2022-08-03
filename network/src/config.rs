use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_BOOTSTRAP: &[&str] = &[
    // URSA bootstrap nodes
    "/ip4/159.223.211.234/tcp/6009/p2p/12D3KooWMd2nE1v5msaRDn9oU9HudRb85X4ED2oKGurH2SntZ4wY",
    "/ip4/146.190.232.131/tcp/6009/p2p/12D3KooWA6LcxGtUhPPe3XnkvAagfqTkWhUL4oqoPBVhNBnLYbNY",
    // "/ip4/0.0.0.0/tcp/4001udp/4001/quic/p2p/Qm",
];

pub const DEFAULT_DB_PATH_STR: &'static str = "ursa_db";

/// Ursa Configration
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct UrsaConfig {
    /// Optional mdns local discovery.
    pub mdns: bool,
    /// Optional relay through other peers.
    pub relay: bool,
    /// Optional autonat.
    pub autonat: bool,
    /// Swarm listening Address.
    pub swarm_addr: Multiaddr,
    /// Bootstrap nodes.
    pub bootstrap_nodes: Vec<Multiaddr>,
    /// Database path.
    pub database_path: Option<PathBuf>,
}

impl Default for UrsaConfig {
    fn default() -> Self {
        let bootstrap_nodes = DEFAULT_BOOTSTRAP
            .iter()
            .map(|node| node.parse().unwrap())
            .collect();

        Self {
            mdns: false,
            relay: false,
            autonat: false,
            bootstrap_nodes,
            swarm_addr: "/ip4/0.0.0.0/tcp/6009".parse().unwrap(),
            database_path: Some(PathBuf::from(DEFAULT_DB_PATH_STR)),
        }
    }
}

impl UrsaConfig {
    pub fn merge(self, other: UrsaConfig) -> Self {
        Self {
            mdns: self.mdns | other.mdns,
            relay: self.relay | other.relay,
            autonat: self.autonat | other.autonat,
            swarm_addr: self.swarm_addr,
            bootstrap_nodes: self.bootstrap_nodes,
            database_path: self.database_path.or(other.database_path),
        }
    }
}
