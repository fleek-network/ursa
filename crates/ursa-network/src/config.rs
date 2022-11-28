use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_TRACKER_URL: &str = "http://tracker-dev.ursa.earth:4000";
pub const DEFAULT_BOOTSTRAP: [&'static str; 2] = [
    "/ip4/159.223.211.234/tcp/6009/p2p/12D3KooWDji7xMLia6GAsyr4oiEFD2dd3zSryqNhfxU3Grzs1r9p",
    "/ip4/146.190.232.131/tcp/6009/p2p/12D3KooWGw8vCj9XayJDMXUiox6pCUFm7oVuWkDJeE2H9SDQVEcM",
];

pub const DEFAULT_DB_PATH_STR: &str = "ursa_db";
pub const DEFAULT_KEYSTORE_PATH_STR: &str = ".config/ursa/keystore";

/// Ursa Configuration
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct UrsaConfig {
    /// Optional mdns local discovery.
    pub mdns: bool,
    /// Optional Provide a relay server for other peers to listen on.
    pub relay_server: bool,
    /// Optional autonat. This is used to determine if we are behind a NAT and need to use a relay.
    pub autonat: bool,
    /// Optional Enable listening on a relay server if not publicly available. Requires autonat.
    /// Connections will attempt to upgrade using dcutr.
    pub relay_client: bool,
    /// Swarm listening Address.
    pub swarm_addr: Multiaddr,
    /// Bootstrap nodes.
    pub bootstrap_nodes: Vec<Multiaddr>,
    /// Database path.
    pub database_path: Option<PathBuf>,
    /// user identity name
    pub identity: String,
    /// Keystore path. Defaults to ~/.config/ursa/keystore
    pub keystore_path: PathBuf,
    /// Temporary HTTP tracker url. This is used for pre-consensus node announcements.
    /// Defaults to devnet tracker.
    pub tracker: Option<String>,
    /// Prometheus metrics port
    pub metrics_port: Option<u16>,
}

impl Default for UrsaConfig {
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
            swarm_addr: "/ip4/0.0.0.0/tcp/6009".parse().unwrap(),
            database_path: Some(PathBuf::from(DEFAULT_DB_PATH_STR)),
            identity: "default".to_string(),
            keystore_path: PathBuf::from(env!("HOME")).join(DEFAULT_KEYSTORE_PATH_STR),
            tracker: Some(DEFAULT_TRACKER_URL.parse().unwrap()),
            metrics_port: Some(4070),
        }
    }
}

impl UrsaConfig {
    pub fn merge(self, other: UrsaConfig) -> Self {
        Self {
            mdns: self.mdns | other.mdns,
            autonat: self.autonat | other.autonat,
            relay_client: self.relay_client | other.relay_client,
            relay_server: self.relay_server | other.relay_server,
            identity: self.identity,
            swarm_addr: self.swarm_addr,
            bootstrap_nodes: self.bootstrap_nodes,
            database_path: self.database_path.or(other.database_path),
            keystore_path: self.keystore_path,
            tracker: self.tracker.or(other.tracker),
            metrics_port: self.metrics_port.or(other.metrics_port),
        }
    }
}
