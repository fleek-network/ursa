use serde::{Deserialize, Serialize};
use ursa_index_provider::config::ProviderConfig;
use ursa_metrics::config::MetricsServiceConfig;
use ursa_network::NetworkConfig;
use ursa_rpc_server::config::ServerConfig;

use std::{
    fs::{create_dir_all, File},
    io::{Result, Write},
    path::PathBuf,
};

pub const DEFAULT_CONFIG_PATH_STR: &str = ".ursa/config.toml";

pub fn load_config(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        let ursa_config = UrsaConfig::default();
        let toml = toml::to_string(&ursa_config).unwrap();
        create_dir_all(path.parent().unwrap())?;
        let mut file = File::create(path)?;
        return file.write_all(toml.as_bytes());
    }
    Ok(())
}

#[derive(Default, Serialize, Deserialize)]
pub struct UrsaConfig {
    pub network_config: NetworkConfig,
    pub provider_config: ProviderConfig,
    pub metrics_config: MetricsServiceConfig,
    pub server_config: ServerConfig,
}
