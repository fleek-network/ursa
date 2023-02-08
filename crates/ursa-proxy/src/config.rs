use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs::read_to_string, path::PathBuf};

pub const DEFAULT_URSA_PROXY_CONFIG_PATH: &str = ".ursa/proxy/config.toml";

pub fn load_config(path: &PathBuf) -> Result<ProxyConfig> {
    if !path.exists() {
        bail!("Could not find config file")
    }
    let toml = read_to_string(path)?;
    toml::from_str(&toml).context("Failed to deserialize")
}

#[derive(Deserialize, Serialize, Default)]
pub struct ProxyConfig {
    pub server: Vec<ServerConfig>,
    pub moka: Option<MokaConfig>,
    pub admin: Option<AdminConfig>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ServerConfig {
    pub proxy_pass: String,
    pub listen_addr: String,
    pub cert_path: String,
    pub key_path: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MokaConfig {
    pub max_capacity: u64,
    pub stream_buf: u64,
    pub time_to_idle: u64,
    pub time_to_live: u64,
}

impl Default for MokaConfig {
    fn default() -> Self {
        Self {
            max_capacity: 200_000_000,   //  Number of entries.
            stream_buf: 1_000_000_000,   //  1GB.
            time_to_idle: 5 * 60 * 1000, //  5 mins.
            time_to_live: 5 * 60 * 1000, //  5 mins.
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct AdminConfig {
    pub addr: String,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            addr: "0.0.0.0:8881".to_string(),
        }
    }
}
