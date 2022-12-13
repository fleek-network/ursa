use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use dirs::home_dir;

const DEFAULT_DB_PATH_STR: &str = ".ursa/data/index_provider_db";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderConfig {
    /// local address
    pub local_address: String,
    /// port where provider is listening
    pub port: u16,
    /// a domain where provider is listening dns/test-node.provider.ursa.earth
    pub domain: String,
    /// indexer url to point to e.g. https://dev.cid.contact
    pub indexer_url: String,
    /// database_path for index provider db
    pub database_path: PathBuf,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            local_address: "0.0.0.0".to_string(),
            port: 8070,
            domain: "".to_string(),
            indexer_url: "https://dev.cid.contact".to_string(),
            database_path: PathBuf::from(home_dir().unwrap_or_default().join(DEFAULT_DB_PATH_STR),
        }
    }
}
