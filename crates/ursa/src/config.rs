use resolve_path::PathResolveExt;
use serde::{Deserialize, Serialize};
use std::{
    fs::{create_dir_all, File},
    io::{Read, Result, Write},
    path::{Path, PathBuf},
    process,
};
use tracing::error;
use ursa_index_provider::config::ProviderConfig;
use ursa_metrics::config::MetricsServiceConfig;
use ursa_network::NetworkConfig;
use ursa_rpc_server::config::ServerConfig;

pub const DEFAULT_CONFIG_PATH_STR: &str = ".ursa/config.toml";

pub fn load_config(path: &PathBuf) -> Result<UrsaConfig> {
    if !path.exists() {
        let ursa_config = UrsaConfig::default();
        let toml = toml::to_string(&ursa_config).unwrap();
        create_dir_all(path.parent().unwrap())?;
        let mut file = File::create(path)?;
        file.write_all(toml.as_bytes())?;
        Ok(ursa_config)
    } else {
        // Read from config file
        let toml = read_file_to_string(path)?;
        // Parse and return the configuration file
        let mut config: UrsaConfig = toml::from_str(&toml)?;
        // parse relative and home directory paths
        config.network_config.keystore_path =
            config.network_config.keystore_path.resolve().to_path_buf();
        config.network_config.database_path =
            config.network_config.database_path.resolve().to_path_buf();
        config.provider_config.database_path =
            config.provider_config.database_path.resolve().to_path_buf();

        Ok(config)
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
