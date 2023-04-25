use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplicationConfig {
    /// The address the application is listening on. defaults to "0.0.0.0:8003".
    #[serde(default = "ApplicationConfig::default_uds")]
    pub abci_uds: PathBuf,
}

impl ApplicationConfig {
    fn default_uds() -> PathBuf {
        "~/.ursa/abci.sock".into()
    }
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            abci_uds: Self::default_uds(),
        }
    }
}
