use serde::{Deserialize, Serialize};

const DEFAULT_METRICS_PORT: u16 = 4070;
const DEFAULT_AGENT: &str = "ursa/*";

#[derive(Serialize, Deserialize, Debug)]
pub struct MetricsServiceConfig {
    /// Optional. Port to run metrics server. Default port 4070.
    pub port: u16,
}

impl Default for MetricsServiceConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_METRICS_PORT,
        }
    }
}
