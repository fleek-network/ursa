use std::{
    fs::{create_dir_all, read_to_string, File},
    io::Write,
    path::PathBuf,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, Level};

use crate::cli::DaemonCmdOpts;

pub const DEFAULT_URSA_GATEWAY_PATH: &str = ".ursa/gateway";
pub const DEFAULT_URSA_GATEWAY_CONFIG_PATH: &str = ".ursa/gateway/config.toml";

const CERT_PEM_CONTENT: &str = include_str!("../self_signed_certs/cert.pem");
const KEY_PEM_CONTENT: &str = include_str!("../self_signed_certs/key.pem");

fn create_cert(cert_pem: &PathBuf, key_pem: &PathBuf) -> Result<()> {
    let mut cert = File::create(cert_pem)?;
    cert.write_all(CERT_PEM_CONTENT.as_bytes())?;
    let mut key = File::create(key_pem)?;
    key.write_all(KEY_PEM_CONTENT.as_bytes())?;
    Ok(())
}

pub fn init_config(path: &PathBuf) -> Result<()> {
    // privilege log
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .finish();
    if !path.exists() {
        return tracing::subscriber::with_default(subscriber, || {
            info!("Create config at: {path:?}");
            let parent_dir = path
                .parent()
                .with_context(|| format!("Couldn't get parent dir from: {path:?}"))?;
            create_dir_all(parent_dir)?;
            let gateway_config = GatewayConfig::default();
            let mut file = File::create(path)?;
            let toml = toml::to_string(&gateway_config)?;
            file.write_all(toml.as_bytes())?;
            info!("Init default self signed tls");
            create_cert(
                &gateway_config.server.cert_path,
                &gateway_config.server.key_path,
            )
            .context("Failed to ensure tls")?;
            Ok(())
        });
    }
    Ok(())
}

pub fn load_config(path: &PathBuf) -> Result<GatewayConfig> {
    // privilege log
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::with_default(subscriber, || info!("Load config at: {:?}", path));
    let toml = read_to_string(path)?;
    toml::from_str(&toml).context("Failed to deserialize")
}

#[derive(Deserialize, Serialize)]
pub struct GatewayConfig {
    pub log_level: String,
    pub server: ServerConfig,
    pub indexer: IndexerConfig,
}

#[derive(Deserialize, Serialize)]
pub struct ServerConfig {
    pub port: u16,
    pub addr: String,
    pub request_timeout: u64,
    pub concurrency_limit: u32,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub stream_buf: u64,
    pub cache_max_capacity: u64,
    pub cache_time_to_idle: u64,
    pub cache_time_to_live: u64,
}

#[derive(Deserialize, Serialize)]
pub struct IndexerConfig {
    pub cid_url: String,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            log_level: "INFO".into(),
            server: ServerConfig {
                addr: "0.0.0.0".into(),
                port: 443,
                request_timeout: 5_000, // 5s
                concurrency_limit: 100_000,
                cert_path: PathBuf::from(env!("HOME"))
                    .join(DEFAULT_URSA_GATEWAY_PATH)
                    .join("cert.pem"),
                key_path: PathBuf::from(env!("HOME"))
                    .join(DEFAULT_URSA_GATEWAY_PATH)
                    .join("key.pem"),
                stream_buf: 2_000_000,             // 2MB
                cache_max_capacity: 100_000,       //  Number of entries.
                cache_time_to_idle: 5 * 60 * 1000, //  5 mins.
                cache_time_to_live: 5 * 60 * 1000, //  5 mins.
            },
            indexer: IndexerConfig {
                cid_url: "https://cid.contact/cid".into(),
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
        if let Some(port) = config.server_port {
            self.server.port = port;
        }
        if let Some(addr) = config.server_addr {
            self.server.addr = addr;
        }
        if let Some(request_timeout) = config.request_timeout {
            self.server.request_timeout = request_timeout;
        }
        if let Some(concurrency_limit) = config.concurrency_limit {
            self.server.concurrency_limit = concurrency_limit;
        }
        if let Some(tls_cert_path) = config.tls_cert_path {
            self.server.cert_path = tls_cert_path;
        }
        if let Some(tls_key_path) = config.tls_key_path {
            self.server.key_path = tls_key_path;
        }
        if let Some(server_stream_buffer) = config.server_stream_buffer {
            self.server.stream_buf = server_stream_buffer;
        }
        if let Some(indexer_cid_url) = config.indexer_cid_url {
            self.indexer.cid_url = indexer_cid_url;
        }
        if let Some(cache_max_capacity) = config.cache_max_capacity {
            self.server.cache_max_capacity = cache_max_capacity;
        }
        if let Some(cache_time_to_live) = config.cache_time_to_live {
            self.server.cache_time_to_live = cache_time_to_live;
        }
        if let Some(cache_time_to_idle) = config.cache_time_to_idle {
            self.server.cache_time_to_idle = cache_time_to_idle;
        }
    }
}
