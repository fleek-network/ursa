use libp2p::Multiaddr;

pub const DEFAULT_BOOTSTRAP: &[&str] = &[
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt",
    "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ",
    // mars.i.ipfs.io
    // once we have quic support in libp2p, we shoul uncomment below
    // "/ip4/104.131.131.82/udp/4001/quic/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ", // mars.i.ipfs.io
];

/// Ursa Configration
#[derive(Debug)]
pub struct UrsaConfig {
    /// Quic Config.
    pub quic: bool,
    /// Optional mdns local discovery.
    pub mdns: bool,
    /// Optional relay through other peers.
    pub relay: bool,
    /// Optional autonat.
    pub autonat: bool,
    /// Listen address
    pub listen: String,
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

        UrsaConfig {
            quic: true,
            mdns: false,
            relay: false,
            autonat: false,
            bootstrap_nodes,
            listen: "0.0.0.0:4020".parse().unwrap(),
            // once quic support
            // "/ip4/0.0.0.0/udp/0/quic".parse().unwrap(),
            swarm_addr: "/ip4/0.0.0.0/tcp/6009".parse().unwrap(),
        }
    }
}
