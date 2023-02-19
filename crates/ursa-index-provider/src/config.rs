use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ProviderConfig {
    /// indexer url to point to, eg. https://dev.cid.contact
    pub indexer_url: String,
    /// database_path for index provider db
    pub database_path: PathBuf,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            indexer_url: "https://dev.cid.contact".to_string(),
            database_path: "~/.ursa/data/index_provider_db".into(),
        }
    }
}
