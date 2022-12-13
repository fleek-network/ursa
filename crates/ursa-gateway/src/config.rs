use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, Level};

use std::{
    fs::{create_dir_all, read_to_string, File},
    io::Write,
    path::PathBuf,
};

use crate::cli::DaemonCmdOpts;

pub const DEFAULT_URSA_GATEWAY_PATH: &str = ".ursa/gateway";
pub const DEFAULT_URSA_GATEWAY_CONFIG_PATH: &str = ".ursa/gateway/config.toml";

pub fn init_config(path: &PathBuf) -> Result<()> {
    // privilege log
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .finish();
    if !path.exists() {
        tracing::subscriber::with_default(subscriber, || info!("create config at: {:?}", path));
        let parent_dir = path
            .parent()
            .with_context(|| format!("couldn't get parent dir from: {:?}", path))?;
        create_dir_all(parent_dir)?;
        let gateway_config = GatewayConfig::default();
        let mut file = File::create(path)?;
        let toml = toml::to_string(&gateway_config)?;
        file.write_all(toml.as_bytes())?;
    }
    Ok(())
}

pub fn load_config(path: &PathBuf) -> Result<GatewayConfig> {
    // privilege log
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::with_default(subscriber, || info!("load config at: {:?}", path));
    let toml = read_to_string(path)?;
    toml::from_str(&toml).context("failed to deserialize")
}

#[derive(Clone, Deserialize, Serialize)]
pub struct GatewayConfig {
    pub log_level: String,
    pub server: ServerConfig,
    pub cert: CertConfig,
    pub indexer: IndexerConfig,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct CertConfig {
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub port: u16,
    pub addr: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct IndexerConfig {
    pub cid_url: String,
    /*
     * pub mh_url: String,
     */
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            log_level: "info".into(),
            server: ServerConfig {
                addr: "0.0.0.0".into(),
                port: 80,
            },
            cert: CertConfig {
                cert_path: PathBuf::from(env!("HOME"))
                    .join(DEFAULT_URSA_GATEWAY_PATH)
                    .join("cert.pem"),
                key_path: PathBuf::from(env!("HOME"))
                    .join(DEFAULT_URSA_GATEWAY_PATH)
                    .join("key.pem"),
            },
            indexer: IndexerConfig {
                cid_url: "https://cid.contact/cid".into(),
                /*
                 * mh_url: "https://cid.contact/multihash".into(),
                 */
            },
        }
    }
}

impl GatewayConfig {
    pub fn merge_log_level(&mut self, log_level: Option<Level>) {
        if let Some(log_level) = log_level {
            self.log_level = log_level.to_string();
        }
    }
    pub fn merge_daemon_opts(&mut self, config: DaemonCmdOpts) {
        if let Some(port) = config.port {
            self.server.port = port;
        }
        if let Some(addr) = config.addr {
            self.server.addr = addr;
        }
        if let Some(tls_cert_path) = config.tls_cert_path {
            self.cert.cert_path = tls_cert_path;
        }
        if let Some(tls_key_path) = config.tls_key_path {
            self.cert.key_path = tls_key_path;
        }
        if let Some(indexer_cid_url) = config.indexer_cid_url {
            self.indexer.cid_url = indexer_cid_url;
        }
        /*
         * if let Some(indexer_mh_url) = config.indexer_mh_url {
         *     self.indexer.mh_url = indexer_mh_url;
         * }
         */
    }
}
