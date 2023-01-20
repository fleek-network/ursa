use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs::read_to_string, path::PathBuf};

use crate::cli::DaemonCmdOpts;

pub const DEFAULT_URSA_PROXY_CONFIG_PATH: &str = ".ursa/proxy/config.toml";

pub fn load_config(path: &PathBuf) -> Result<ProxyConfig> {
    let toml = read_to_string(path)?;
    toml::from_str(&toml).context("Failed to deserialize")
}

#[derive(Deserialize, Serialize, Default)]
pub struct ProxyConfig {
    pub server: Vec<ServerConfig>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ServerConfig {
    pub port: u16,
    pub addr: String,
    pub listen_addr: Option<String>,
    pub listen_port: Option<u16>,
}
