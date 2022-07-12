use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};

pub const DEFAULT_BOOTSTRAP: &[&str] = &[
    // URSA bootstrap nodes
    // "/ip4/0.0.0.0/tcp/4001/p2p/Qm",
    // "/ip4/0.0.0.0/tcp/4001/p2p/Qm",
    // "/ip4/0.0.0.0/tcp/4001udp/4001/quic/p2p/Qm",
    "/ip4/127.0.0.1/tcp/6009",
];

pub const DEFAULT_DATABASE_PATH: &str = "ursa_db";

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
        }
    }
}
