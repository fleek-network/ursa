use libp2p::Multiaddr;

pub const DEFAULT_BOOTSTRAP: &[&str] = &[
    // URSA bootstrap nodes
    // "/ip4/0.0.0.0/tcp/4001/p2p/Qm",
    // "/ip4/0.0.0.0/tcp/4001/p2p/Qm",
    // "/ip4/0.0.0.0/tcp/4001udp/4001/quic/p2p/Qm",

    // IPFS bootstrap nodes
    // "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
    // "/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
    // "/dnsaddr/bootstrap.libp2p.io/p2p/QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
    // "/dnsaddr/bootstrap.libp2p.io/p2p/QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt",
    // "/ip4/104.131.131.82/tcp/4001/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ",
    // once we have quic support in libp2p, we shoul uncomment below
    // "/ip4/104.131.131.82/udp/4001/quic/p2p/QmaCpDMGvV2BGHeYERUEnRQAwe3N8SzbUtfsmvsqQLuvuJ", // mars.i.ipfs.io
    // "/dns4/bootstrap-1.interop.fildev.network/tcp/1347/p2p/12D3KooWL8YeT6dDpfushm4Y1LeZjvG1dRMbs8JUERoF4YvxDqfD",
    "/ip4/127.0.0.1/tcp/6009",
];

/// Ursa Configration
#[derive(Debug, Clone)]
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
            // once quic support
            // "/ip4/0.0.0.0/udp/0/quic".parse().unwrap(),
            swarm_addr: "/ip4/0.0.0.0/tcp/6009".parse().unwrap(),
        }
    }
}
