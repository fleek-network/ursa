use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ApplicationConfig {
    ///The address the application is listening on. defaults to "0.0.0.0:8003"
    pub domain: String,
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            domain: "0.0.0.0:8003".into(),
        }
    }
}
