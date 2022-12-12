use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::info;

use std::{
    fs::{create_dir_all, read_to_string, File},
    io::Write,
    path::PathBuf,
};

pub const DEFAULT_URSA_GATEWAY_PATH: &str = ".ursa/gateway";
pub const DEFAULT_URSA_GATEWAY_CONFIG_PATH: &str = ".ursa/gateway/config.toml";

pub fn init_config(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        info!("create config at: {:?}", path);
        let parent_dir = path
            .parent()
            .with_context(|| format!("couldn't get parent dir from: {:?}", path))?;
        create_dir_all(parent_dir)?;
        let gateway_config = GatewayConfig::default();
        let mut file = File::create(path)?;
        let toml = toml::to_string(&gateway_config)?;
        file.write_all(toml.as_bytes())?;
    }
    Ok(())
}

pub fn load_config(path: &PathBuf) -> Result<GatewayConfig> {
    info!("load config at: {:?}", path);
    let toml = read_to_string(path)?;
    toml::from_str(&toml).context("failed to deserialize")
}

#[derive(Clone, Deserialize, Serialize)]
pub struct GatewayConfig {
    pub server: ServerConfig,
    pub cert: CertConfig,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct CertConfig {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub port: u16,
    pub addr: String,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                addr: "0.0.0.0".into(),
                port: 80,
            },
            cert: CertConfig {
                cert_path: PathBuf::from(env!("HOME"))
                    .join(DEFAULT_URSA_GATEWAY_PATH)
                    .join("cert.pem"),
                key_path: PathBuf::from(env!("HOME"))
                    .join(DEFAULT_URSA_GATEWAY_PATH)
                    .join("key.pem"),
            },
        }
    }
}
