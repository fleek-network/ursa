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
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ServerConfig {
    pub proxy_pass: String,
    pub listen_addr: Option<String>,
    pub listen_port: Option<u16>,
}
