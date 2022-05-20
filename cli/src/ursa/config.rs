
// use libp2p::identity::Keypair;
use libp2p::{Multiaddr, PeerId};

/// Fnet Configration
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, PartialEq)]
/// Fnet Configration
pub struct CliConfig {
    /// Node key
    // pub keypair: Keypair,
    /// Swarm listening Address
    /// "/ip4/0.0.0.0/udp/0/quic".parse().unwrap()
    pub swarm_addr: Multiaddr,
    /// Quic Config
    pub quic: bool,
    // Bootstrap nodes
    pub bootstrap_nodes: Vec<(PeerId, Multiaddr)>,
}

impl Default for CliConfig {
    fn default() -> Self {
        CliConfig {
            // keypair: todo!(),
            swarm_addr: "/ip4/0.0.0.0/udp/0/quic".parse().unwrap(),
            quic: true,
            bootstrap_nodes: vec![],
        }
    }
}
impl CliConfig {
    pub fn new() {
        todo!();
    }
}