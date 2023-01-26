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
    pub tlrfu: Option<TlrfuConfig>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ServerConfig {
    pub proxy_pass: String,
    pub listen_addr: Option<String>,
    pub listen_port: Option<u16>,
    pub no_cache: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TlrfuConfig {
    pub max_size: u64,
    pub ttl_buf: u128,
    pub stream_buf: u64,
}

impl Default for TlrfuConfig {
    fn default() -> Self {
        Self {
            max_size: 200_000_000,  // 200MB
            ttl_buf: 5 * 60 * 1000, // 5 mins
            stream_buf: 2_000_000,  // 2MB
        }
    }
}
