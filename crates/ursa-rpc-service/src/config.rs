use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct ServerConfig {
    #[serde(default = "ServerConfig::default_port")]
    pub port: u16,
    #[serde(default = "ServerConfig::default_addr")]
    pub addr: String,
}

impl ServerConfig {
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
            port: Self::default_port(),
            addr: Self::default_addr(),
        }
    }
}
