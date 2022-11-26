#[derive(Clone)]
pub struct GatewayConfig {
    pub server: ServerConfig,
    pub reverse_proxy: ServerConfig,
    pub cert_config: CertConfig,
}

#[derive(Clone)]
pub struct CertConfig {
    pub cert_path: String,
    pub key_path: String,
}

#[derive(Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub addr: String,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                addr: "0.0.0.0".to_string(),
                port: 3000,
            },
            reverse_proxy: ServerConfig {
                addr: "0.0.0.0".to_string(),
                port: 4000,
            },
            cert_config: CertConfig {
                // TODO: move out to better location.
                cert_path: "./crates/ursa-gateway/self_signed_certs/cert.pem".to_string(),
                key_path: "./crates/ursa-gateway/self_signed_certs/key.pem".to_string(),
            },
        }
    }
}
