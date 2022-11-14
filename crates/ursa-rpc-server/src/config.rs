use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct ServerConfig {
    pub port: u16,
    pub addr: String,
}

impl ServerConfig {
    pub fn new(port: u16, addr: String) -> Self {
        Self { port, addr }
    }
}
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 4069,
            addr: "0.0.0.0".to_string(),
        }
    }
}
