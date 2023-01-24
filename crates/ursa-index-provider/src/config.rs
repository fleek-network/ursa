use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderConfig {
    /// ! Deprecated ! Moved to `ursa-rpc-service` config.
    /// Left here for backwards compatability; so that configs can be migrated
    #[serde(default)]
    pub domain: Option<String>,
    /// indexer url to point to, eg. https://dev.cid.contact
    #[serde(default = "ProviderConfig::default_indexer_url")]
    pub indexer_url: String,
    /// database_path for index provider db
    #[serde(default = "ProviderConfig::default_database_path")]
    pub database_path: PathBuf,
}

impl ProviderConfig {
    fn default_database_path() -> PathBuf {
        "~/.ursa/data/index_provider_db".into()
    }
    fn default_indexer_url() -> String {
        "https://dev.cid.contact".to_string()
    }
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            domain: None,
            indexer_url: Self::default_indexer_url(),
            database_path: Self::default_database_path(),
        }
    }
}
