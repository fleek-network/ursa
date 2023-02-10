use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct ServerConfig {
    /// Public IP address of the node, eg. `/ip4/127.0.0.1`
    #[serde(default = "ServerConfig::default_addresses")]
    pub addresses: Vec<Multiaddr>,
    /// Port to listen on
    #[serde(default = "ServerConfig::default_port")]
    pub port: u16,
    /// Address to bind to
    #[serde(default = "ServerConfig::default_addr")]
    pub addr: String,
    #[serde(default)]
    pub origin: OriginConfig,
}

impl ServerConfig {
    fn default_addresses() -> Vec<Multiaddr> {
        vec!["/ip4/127.0.0.1/tcp/4069".parse().unwrap()]
    }
    fn default_port() -> u16 {
        4069
    }
    fn default_addr() -> String {
        "0.0.0.0".to_string()
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            addresses: Self::default_addresses(),
            port: Self::default_port(),
            addr: Self::default_addr(),
            origin: Default::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct OriginConfig {
    /// Ipfs gateway url
    #[serde(default = "OriginConfig::default_ipfs_gateway")]
    pub ipfs_gateway: String,
    /// Intended for testing purposes
    pub use_https: Option<bool>,
}

impl OriginConfig {
    pub fn default_ipfs_gateway() -> String {
        "ipfs.io".to_string()
    }
}

impl Default for OriginConfig {
    fn default() -> Self {
        Self {
            ipfs_gateway: Self::default_ipfs_gateway(),
            use_https: None,
        }
    }
}
