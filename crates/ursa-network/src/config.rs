use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_TRACKER_URL: &str = "https://tracker.ursa.earth/announce";
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
    pub swarm_addr: Multiaddr,
    /// Bootstrap nodes.
    pub bootstrap_nodes: Vec<Multiaddr>,
    /// Database path.
    pub database_path: PathBuf,
    /// user identity name
    pub identity: String,
    /// Keystore path. Defaults to ~/.ursa/keystore
    pub keystore_path: PathBuf,
    /// Temporary HTTP tracker url. This is used for pre-consensus node announcements.
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
            bootstrapper: false,
            swarm_addr: "/ip4/0.0.0.0/tcp/6009".parse().unwrap(),
            database_path: PathBuf::from(env!("HOME")).join(DEFAULT_DB_PATH_STR),
            identity: "default".to_string(),
            keystore_path: PathBuf::from(env!("HOME")).join(DEFAULT_KEYSTORE_PATH_STR),
            tracker: DEFAULT_TRACKER_URL.into(),
        }
    }
}
