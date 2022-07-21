use serde::Deserialize;

#[derive(Deserialize)]
pub struct RpcConfig {
    pub rpc_port: u16,
    pub rpc_addr: String,
}

impl RpcConfig {
    pub fn new(rpc_port: u16, rpc_addr: String) -> Self {
        Self { rpc_port, rpc_addr }
    }
}
impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            rpc_port: 4069,
            rpc_addr: "0.0.0.0".to_string(),
        }
    }
}
