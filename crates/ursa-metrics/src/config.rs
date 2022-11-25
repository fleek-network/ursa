use serde::{Deserialize, Serialize};

const DEFAULT_METRICS_PORT: &str = "4070";
const DEFAULT_API_PATH: &str = "/metrics";

#[derive(Serialize, Deserialize)]
pub struct MetricsServiceConfig {
    /// Optional. Port to run metrics server. Default port 4070.
    pub port: String,
    /// Options. Path to metrics. Default /metrics
    pub api_path: String,
}

impl Default for MetricsServiceConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_METRICS_PORT.to_string(),
            api_path: DEFAULT_API_PATH.to_string(),
        }
    }
}
