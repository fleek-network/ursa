use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderConfig {
    /// a domain where the node is listening, eg. `dns/test-node.ursa.earth`
    #[serde(default)]
    pub domain: String,
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
            domain: "".to_string(),
            indexer_url: Self::default_indexer_url(),
            database_path: Self::default_database_path(),
        }
    }
}
