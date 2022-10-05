const DEFAULT_METRICS_PORT: &str = "4070";
const DEFAULT_API_PATH: &str = "/metrics";

#[derive(Debug)]
pub struct MetricsServiceConfig {
    /// Optional. Port to run metrics server. Default port 4070.
    pub port: u16,
    /// Options. Path to metrics. Default /metrics
    pub api_path: String,
}

impl MetricsServiceConfig {
    pub fn new(port: u16, api_path: String) -> Self {
        Self {
            port,
            api_path,
        }
    }
}

impl Default for MetricsServiceConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_METRICS_PORT.parse().unwrap(),
            api_path: DEFAULT_API_PATH.to_string(),
        }
    }
}
