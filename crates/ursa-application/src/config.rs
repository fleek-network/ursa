use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplicationConfig {
    ///The address the application is listening on. defaults to "0.0.0.0:8003"
    #[serde(default = "ApplicationConfig::default_domain")]
    pub domain: String,
}

impl ApplicationConfig {
    fn default_domain() -> String {
        "0.0.0.0:8003".into()
    }
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            domain: Self::default_domain(),
        }
    }
}
