use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
#[serde(default)]
pub struct ServerConfig {
    /// Public multiaddresses of the node,
    /// eg. `/dns/node.user.domain/tcp/4069` or `/ip4/1.2.3.4/tcp/4069`
    pub addresses: Vec<Multiaddr>,
    /// Port to listen on
    pub port: u16,
    /// Address to bind to
    pub addr: String,
    /// Origin fallback configuration
    pub origin: OriginConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            addresses: vec!["/ip4/127.0.0.1/tcp/4069".parse().unwrap()],
            port: 4069,
            addr: "0.0.0.0".to_string(),
            origin: Default::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(default)]
pub struct OriginConfig {
    /// Ipfs gateway url
    pub ipfs_gateway: String,
    /// Intended for testing purposes
    pub use_https: Option<bool>,
}

impl Default for OriginConfig {
    fn default() -> Self {
        Self {
            ipfs_gateway: "ipfs.io".to_string(),
            use_https: None,
        }
    }
}
