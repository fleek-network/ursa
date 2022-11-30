const DEFAULT_METRICS_PORT: u16 = 4070;
const DEFAULT_API_PATH: &str = "/metrics";
const DEFAULT_AGENT: &str = "ursa/*";

pub struct MetricsServiceConfig {
    /// Optional. Port to run metrics server. Default port 4070.
    pub port: u16,
    /// Options. Path to metrics. Default /metrics
    pub api_path: String,
    /// Ursa client version
    pub agent: String,
}

impl Default for MetricsServiceConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_METRICS_PORT,
            api_path: DEFAULT_API_PATH.to_string(),
            agent: DEFAULT_AGENT.to_string(),
        }
    }
}
