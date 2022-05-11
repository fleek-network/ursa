use libp2p::gossipsub::GossipsubConfig;
use libp2p::identity::Keypair;
use libp2p::kad::store::MemoryStoreConfig as KademliaConfig;
use libp2p::ping::Config as PingConfig;
use libp2p::{Multiaddr, PeerId};

/// Fnet Configration
#[derive(Debug)]
pub struct FnetConfig {
    /// Node key
    pub keypair: Keypair,
    /// Swarm listening Address
    /// "/ip4/0.0.0.0/udp/0/quic".parse().unwrap()
    pub swarm_addr: Multiaddr,
    /// Quic Config
    // quic: QuicConfig,
    /// Kad Config
    pub kademlia: KademliaConfig,
    /// Ping Config
    pub ping: PingConfig,
    /// Gossip Config
    pub gossip: GossipsubConfig,
    // Bootstrap nodes
    pub bootstrap_nodes: Vec<(PeerId, Multiaddr)>,
}

impl FnetConfig {
    pub fn new() {
        todo!()
    }
}
