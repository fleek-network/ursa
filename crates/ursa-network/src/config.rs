use dirs::home_dir;
use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_TRACKER_URL: &str = "https://tracker.ursa.earth/register";
pub const DEFAULT_BOOTSTRAP: [&str; 2] = [
    "/ip4/159.223.211.234/tcp/6009/p2p/12D3KooWDji7xMLia6GAsyr4oiEFD2dd3zSryqNhfxU3Grzs1r9p",
    "/ip4/146.190.232.131/tcp/6009/p2p/12D3KooWGw8vCj9XayJDMXUiox6pCUFm7oVuWkDJeE2H9SDQVEcM",
];
pub const DEFAULT_DB_PATH_STR: &str = ".ursa/data/ursa_db";
pub const DEFAULT_KEYSTORE_PATH_STR: &str = ".ursa/keystore";

/// Ursa Configuration
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
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
    /// Interval (in seconds) to run kademlia bootstraps at. Default is 300 (5 minutes).
    pub bootstrap_interval: u64,
    /// Database path.
    pub database_path: PathBuf,
    /// user identity name
    pub identity: String,
    /// Keystore path. Defaults to ~/.ursa/keystore
    pub keystore_path: PathBuf,
    /// Temporary HTTP tracker url. This is used for pre-consensus node registrations.
    /// Defaults to devnet tracker.
    pub tracker: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        let bootstrap_nodes = DEFAULT_BOOTSTRAP
            .iter()
            .map(|node| node.parse().unwrap())
            .collect();

        Self {
            mdns: false,
            autonat: true,
            relay_client: true,
            relay_server: true,
            bootstrap_nodes,
            bootstrap_interval: 300, // 5 minutes
            bootstrapper: false,
            swarm_addrs: vec![
                "/ip4/0.0.0.0/tcp/6009".parse().unwrap(),
                "/ip4/0.0.0.0/udp/4890/quic-v1".parse().unwrap(),
            ],
            database_path: home_dir().unwrap_or_default().join(DEFAULT_DB_PATH_STR),
            identity: "default".to_string(),
            tracker: DEFAULT_TRACKER_URL.into(),
            keystore_path: home_dir()
                .unwrap_or_default()
                .join(DEFAULT_KEYSTORE_PATH_STR),
        }
    }
}
