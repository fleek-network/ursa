use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const DEFAULT_DB_PATH_STR: &str = ".ursa/data/index_provider_db";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderConfig {
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
            domain: "".to_string(),
            indexer_url: "https://dev.cid.contact".to_string(),
            database_path: format!("~/{DEFAULT_DB_PATH_STR}").into(),
        }
    }
}
