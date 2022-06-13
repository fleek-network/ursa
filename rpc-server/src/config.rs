use serde::Deserialize;

#[derive(Deserialize)]
pub struct RpcConfig {
    pub port: u16,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self { port: 9000 }
    }
}
