use anyhow::{Context, Result};
use imara_diff::{intern::InternedInput, sink::Counter, Algorithm, UnifiedDiffBuilder};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::{
    fs::{create_dir_all, File},
    io::{Read, Write},
    path::PathBuf,
};
use tracing::{info, warn};
use ursa_application::ApplicationConfig;
use ursa_consensus::config::ConsensusConfig;
use ursa_index_provider::config::ProviderConfig;
use ursa_network::NetworkConfig;
use ursa_rpc_service::config::ServerConfig;

pub const DEFAULT_CONFIG_PATH_STR: &str = ".ursa/config.toml";
 
#[derive(Default, Serialize, Deserialize, Debug)]
pub struct UrsaConfig {
    #[serde(default)]
    pub network_config: NetworkConfig,
    #[serde(default)]
    pub provider_config: ProviderConfig,
    #[serde(default)]
    pub server_config: ServerConfig,
    #[serde(default)]
    pub consensus_config: ConsensusConfig,
    #[serde(default)]
    pub application_config: ApplicationConfig,
}

impl UrsaConfig {
    /// Load an UrsaConfig from a given path, or create a default one if not found.
    pub fn load_or_default(path: &PathBuf) -> Result<UrsaConfig> {
        info!("Loading config from: {:?}", path);
        if path.exists() {
            // read file
            let mut file = File::open(path)?;
            let mut raw = String::new();
            file.read_to_string(&mut raw)?;

            let config: UrsaConfig = toml::from_str(&raw).context("Failed to parse config file")?;

            // check if we modified the config at all
            let config_str = toml::to_string(&config)?;
            let input = InternedInput::new(raw.as_str(), config_str.as_str());
            let diff = imara_diff::diff(
                Algorithm::Histogram,
                &input,
                Counter::new(UnifiedDiffBuilder::new(&input)),
            );
            if diff.total() > 0 {
                warn!(
                    "Config `{path:?}` was automatically modified and saved to disk:\n\n{}",
                    diff.wrapped
                );
                write(config_str, path)?;
            }

            Ok(config)
        } else {
            warn!("Config `{path:?}` not found, writing default config");
            // Config missing, create and return default config at path
            let config = UrsaConfig::default();
            write(toml::to_string(&config)?, path)?;

            Ok(config)
        }
    }
}

pub fn write<S: Display, P: Into<PathBuf>>(str: S, path: P) -> Result<()> {
    let path = path.into();
    create_dir_all(path.parent().unwrap())?;
    let mut file = File::create(path)?;
    file.write_all(str.to_string().as_bytes())?;
    Ok(())
}
