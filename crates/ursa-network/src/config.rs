use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Ursa Configuration
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(default)]
pub struct NetworkConfig {
    /// Optional mdns local discovery.
    pub mdns: bool,
    /// Optional Provide a relay server for other peers to listen on.
    pub relay_server: bool,
    /// Optional autonat. This is used to determine if we are behind a NAT and need to use a relay.
    pub autonat: bool,
    /// Optional Enable listening on a relay server if not publicly available. Requires autonat.
    /// Connections will attempt to upgrade using dcutr.
    pub relay_client: bool,
    /// set true if it is a bootstrap node. default = false
    pub bootstrapper: bool,
    /// Swarm listening Address.
    pub swarm_addrs: Vec<Multiaddr>,
    /// Bootstrap nodes.
    pub bootstrap_nodes: Vec<Multiaddr>,
    /// Database path.
    pub database_path: PathBuf,
    /// user identity name
    pub identity: String,
    /// Keystore path. Defaults to ~/.ursa/keystore
    pub keystore_path: PathBuf,
    /// Temporary HTTP tracker url. This is used for pre-consensus node registrations.
    /// Defaults to devnet tracker.
    pub tracker: String,
    /// Determines the number of closest peers to which a record is replicated
    pub kad_replication_factor: usize,
    /// Interval to run random kademlia walks to refresh the routing table. Defaults to 5 minutes
    pub kad_walk_interval: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            mdns: false,
            autonat: true,
            relay_client: true,
            relay_server: true,
            bootstrapper: true,
            bootstrap_nodes: vec![
                "/ip4/159.223.211.234/tcp/6009/p2p/12D3KooWDji7xMLia6GAsyr4oiEFD2dd3zSryqNhfxU3Grzs1r9p".parse().unwrap(),
                "/ip4/146.190.232.131/tcp/6009/p2p/12D3KooWGw8vCj9XayJDMXUiox6pCUFm7oVuWkDJeE2H9SDQVEcM".parse().unwrap(),
            ],
            swarm_addrs: vec![
                "/ip4/0.0.0.0/tcp/6009".parse().unwrap(),
                "/ip4/0.0.0.0/udp/4890/quic-v1".parse().unwrap(),
            ],
            database_path: "~/.ursa/data/ursa_db".into(),
            keystore_path: "~/.ursa/keystore".into(),
            identity: "default".into(), 
            tracker: "https://tracker.ursa.earth/register".into(),
            kad_replication_factor: 8,
            kad_walk_interval: 300,
        }
    }
}
