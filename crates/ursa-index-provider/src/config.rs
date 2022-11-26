use serde::Deserialize;

#[derive(Deserialize)]
pub struct ProviderConfig {
    pub addr: String,
    pub port: u16,
}

impl ProviderConfig {
    pub fn new(addr: String, port: u16) -> Self {
        Self { addr, port }
    }
}
impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            port: 8070,
            addr: "0.0.0.0".to_string(),
        }
    }
}
