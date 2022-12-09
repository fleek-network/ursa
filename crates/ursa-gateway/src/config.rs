use serde::{Deserialize, Serialize};
use tracing::info;

use std::{
    fs::{create_dir_all, read_to_string, File},
    io::Write,
    path::PathBuf,
};

pub const DEFAULT_URSA_GATEWAY_PATH: &str = ".ursa/gateway/";
pub const DEFAULT_URSA_GATEWAY_CONFIG_PATH: &str = ".ursa/gateway/config.toml";

pub fn init_config() {
    let path = PathBuf::from(env!("HOME")).join(DEFAULT_URSA_GATEWAY_CONFIG_PATH);
    if !path.exists() {
        info!("init config path at: {:?}", path);
        create_dir_all(path.parent().expect("create parent config path"))
            .expect("couldn't create parent config path");
        let gateway_config = GatewayConfig::default();
        let mut file = File::create(path).expect("create config path");
        let toml = toml::to_string(&gateway_config).expect("toml serialization");
        file.write_all(toml.as_bytes()).expect("problem write file");
    }
}

pub fn load_config() -> GatewayConfig {
    let path = PathBuf::from(env!("HOME")).join(DEFAULT_URSA_GATEWAY_CONFIG_PATH);
    info!("load config path at: {:?}", path);
    let toml = read_to_string(path).expect("problem read file");
    toml::from_str(&toml).expect("load config failed")
}

#[derive(Clone, Deserialize, Serialize)]
pub struct GatewayConfig {
    pub server: ServerConfig,
    pub cert_config: CertConfig,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct CertConfig {
    pub cert_path: String,
    pub key_path: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub port: u16,
    pub addr: String,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                addr: "0.0.0.0".to_string(),
                port: 80,
            },
            cert_config: CertConfig {
                cert_path: PathBuf::from(env!("HOME"))
                    .join(DEFAULT_URSA_GATEWAY_PATH)
                    .join("cert.pem")
                    .to_str()
                    .expect("bad directory")
                    .into(),
                key_path: PathBuf::from(env!("HOME"))
                    .join(DEFAULT_URSA_GATEWAY_PATH)
                    .join("key.pem")
                    .to_str()
                    .expect("bad directory")
                    .into(),
            },
        }
    }
}
