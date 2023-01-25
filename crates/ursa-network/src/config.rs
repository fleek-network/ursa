use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Ursa Configuration
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct NetworkConfig {
    /// Optional mdns local discovery.
    #[serde(default = "NetworkConfig::default_mdns")]
    pub mdns: bool,
    /// Optional Provide a relay server for other peers to listen on.
    #[serde(default = "NetworkConfig::default_relay_server")]
    pub relay_server: bool,
    /// Optional autonat. This is used to determine if we are behind a NAT and need to use a relay.
    #[serde(default = "NetworkConfig::default_autonat")]
    pub autonat: bool,
    /// Optional Enable listening on a relay server if not publicly available. Requires autonat.
    /// Connections will attempt to upgrade using dcutr.
    #[serde(default = "NetworkConfig::default_relay_client")]
    pub relay_client: bool,
    /// set true if it is a bootstrap node. default = false
    #[serde(default = "NetworkConfig::default_bootstrapper")]
    pub bootstrapper: bool,
    /// Swarm listening Address.
    #[serde(default = "NetworkConfig::default_swarm_addrs")]
    pub swarm_addrs: Vec<Multiaddr>,
    /// Bootstrap nodes.
    #[serde(default = "NetworkConfig::default_bootstrap_nodes")]
    pub bootstrap_nodes: Vec<Multiaddr>,
    /// Database path.
    #[serde(default = "NetworkConfig::default_database_path")]
    pub database_path: PathBuf,
    /// user identity name
    #[serde(default = "NetworkConfig::default_identity")]
    pub identity: String,
    /// Keystore path. Defaults to ~/.ursa/keystore
    #[serde(default = "NetworkConfig::default_keystore_path")]
    pub keystore_path: PathBuf,
    /// Temporary HTTP tracker url. This is used for pre-consensus node registrations.
    /// Defaults to devnet tracker.
    #[serde(default = "NetworkConfig::default_tracker")]
    pub tracker: String,
    /// Determines the number of closest peers to which a record is replicated
    #[serde(default = "NetworkConfig::default_kad_replication_factor")]
    pub kad_replication_factor: usize,
}

impl NetworkConfig {
    fn default_mdns() -> bool {
        false
    }
    fn default_autonat() -> bool {
        true
    }
    fn default_relay_client() -> bool {
        true
    }
    fn default_relay_server() -> bool {
        true
    }
    fn default_bootstrapper() -> bool {
        false
    }
    fn default_tracker() -> String {
        "https://tracker.ursa.earth/register".to_string()
    }
    fn default_bootstrap_nodes() -> Vec<Multiaddr> {
        vec![
            "/ip4/159.223.211.234/tcp/6009/p2p/12D3KooWDji7xMLia6GAsyr4oiEFD2dd3zSryqNhfxU3Grzs1r9p".parse().unwrap(),
            "/ip4/146.190.232.131/tcp/6009/p2p/12D3KooWGw8vCj9XayJDMXUiox6pCUFm7oVuWkDJeE2H9SDQVEcM".parse().unwrap(),
        ]
    }
    fn default_swarm_addrs() -> Vec<Multiaddr> {
        vec![
            "/ip4/0.0.0.0/tcp/6009".parse().unwrap(),
            "/ip4/0.0.0.0/udp/4890/quic-v1".parse().unwrap(),
        ]
    }
    fn default_database_path() -> PathBuf {
        "~/.ursa/data/ursa_db".into()
    }
    fn default_keystore_path() -> PathBuf {
        "~/.ursa/keystore".into()
    }
    fn default_identity() -> String {
        "default".to_string()
    }
    fn default_kad_replication_factor() -> usize {
        8
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            mdns: Self::default_mdns(),
            autonat: Self::default_autonat(),
            relay_client: Self::default_relay_client(),
            relay_server: Self::default_relay_server(),
            bootstrapper: Self::default_bootstrapper(),
            bootstrap_nodes: Self::default_bootstrap_nodes(),
            swarm_addrs: Self::default_swarm_addrs(),
            database_path: Self::default_database_path(),
            identity: Self::default_identity(),
            tracker: Self::default_tracker(),
            keystore_path: Self::default_keystore_path(),
            kad_replication_factor: Self::default_kad_replication_factor(),
        }
    }
}
