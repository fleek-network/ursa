use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::{create_dir_all, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process,
};
use tracing::{error, info};
use ursa_index_provider::config::ProviderConfig;
use ursa_metrics::config::MetricsServiceConfig;
use ursa_network::NetworkConfig;
use ursa_rpc_server::config::ServerConfig;

pub const DEFAULT_CONFIG_PATH_STR: &str = ".ursa/config.toml";

pub fn load_config(path: &PathBuf) -> Result<UrsaConfig> {
    info!("Loading config from: {:?}", path);
    if path.exists() {
        let toml = read_file_to_string(path).context(format!("Failed to read config file {}", path.to_string_lossy()))?;
        toml::from_str(&toml).map_err(|e| anyhow!("Failed to parse config toml: {}", e))
    } else {
        // Missing, create and return default config at path
        let ursa_config = UrsaConfig::default();
        let toml = toml::to_string(&ursa_config).unwrap();
        create_dir_all(path.parent().unwrap()).context(format!("Failed to create default config directory: {}", path.to_string_lossy()))?;
        let mut file = File::create(path).context(format!("Failed to create default config: {}", path.to_string_lossy()))?;
        file.write_all(toml.as_bytes()).context(format!("Failed to write default config: {}", path.to_string_lossy()))?;
        Ok(ursa_config)
    }
}

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct UrsaConfig {
    pub network_config: NetworkConfig,
    pub provider_config: ProviderConfig,
    pub metrics_config: MetricsServiceConfig,
    pub server_config: ServerConfig,
}

/// Read file as a `String`.
pub fn read_file_to_string(path: &Path) -> Result<String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) => {
            error!("Problem opening the file: {:?}", error);
            process::exit(1);
        }
    };
    let mut string = String::new();
    file.read_to_string(&mut string)?;
    Ok(string)
}
