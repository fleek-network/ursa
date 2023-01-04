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

pub fn init_config(path: &PathBuf) -> Result<()> {
    // privilege log
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .finish();
    if !path.exists() {
        tracing::subscriber::with_default(subscriber, || info!("Create config at: {path:?}"));
        let parent_dir = path
            .parent()
            .with_context(|| format!("Couldn't get parent dir from: {path:?}"))?;
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
    tracing::subscriber::with_default(subscriber, || info!("Load config at: {:?}", path));
    let toml = read_to_string(path)?;
    toml::from_str(&toml).context("Failed to deserialize")
}

#[derive(Deserialize, Serialize)]
pub struct GatewayConfig {
    pub log_level: String,
    pub server: ServerConfig,
    pub admin_server: AdminConfig,
    pub indexer: IndexerConfig,
    pub cache: CacheConfig,
    pub worker: WorkerConfig,
}

#[derive(Deserialize, Serialize)]
pub struct ServerConfig {
    pub port: u16,
    pub addr: String,
    pub request_body_limit: u64,
    pub request_timeout: u64,
    pub concurrency_limit: u32,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub stream_buf: u64,
}

#[derive(Deserialize, Serialize)]
pub struct AdminConfig {
    pub port: u16,
    pub addr: String,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
}

#[derive(Deserialize, Serialize)]
pub struct IndexerConfig {
    pub cid_url: String,
    /*
     * pub mh_url: String,
     */
}

#[derive(Deserialize, Serialize)]
pub struct CacheConfig {
    pub max_size: u64,
    pub ttl_buf: u64,
}

#[derive(Deserialize, Serialize)]
pub struct WorkerConfig {
    pub ttl_cache_interval: u64,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            log_level: "INFO".into(),
            server: ServerConfig {
                addr: "0.0.0.0".into(),
                port: 80,
                request_body_limit: 128,
                request_timeout: 5_000,
                concurrency_limit: 100_000,
                cert_path: PathBuf::from(env!("HOME"))
                    .join(DEFAULT_URSA_GATEWAY_PATH)
                    .join("server_cert.pem"),
                key_path: PathBuf::from(env!("HOME"))
                    .join(DEFAULT_URSA_GATEWAY_PATH)
                    .join("server_key.pem"),
                stream_buf: 2_000_000, // 2MB
            },
            admin_server: AdminConfig {
                addr: "0.0.0.0".into(),
                port: 5001,
                cert_path: PathBuf::from(env!("HOME"))
                    .join(DEFAULT_URSA_GATEWAY_PATH)
                    .join("admin_cert.pem"),
                key_path: PathBuf::from(env!("HOME"))
                    .join(DEFAULT_URSA_GATEWAY_PATH)
                    .join("admin_key.pem"),
            },
            indexer: IndexerConfig {
                cid_url: "https://cid.contact/cid".into(),
                /*
                 * mh_url: "https://cid.contact/multihash".into(),
                 */
            },
            cache: CacheConfig {
                max_size: 200_000_000,  // 200MB
                ttl_buf: 5 * 60 * 1000, // 5 mins
            },
            worker: WorkerConfig {
                ttl_cache_interval: 5 * 60 * 1000, // 5 mins
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
        if let Some(request_body_limit) = config.request_body_limit {
            self.server.request_body_limit = request_body_limit;
        }
        if let Some(request_timeout) = config.request_timeout {
            self.server.request_timeout = request_timeout;
        }
        if let Some(concurrency_limit) = config.concurrency_limit {
            self.server.concurrency_limit = concurrency_limit;
        }
        if let Some(tls_cert_path) = config.server_tls_cert_path {
            self.server.cert_path = tls_cert_path;
        }
        if let Some(tls_key_path) = config.server_tls_key_path {
            self.server.key_path = tls_key_path;
        }
        if let Some(server_stream_buffer) = config.server_stream_buffer {
            self.server.stream_buf = server_stream_buffer;
        }
        if let Some(port) = config.admin_port {
            self.admin_server.port = port;
        }
        if let Some(addr) = config.admin_addr {
            self.admin_server.addr = addr;
        }
        if let Some(tls_cert_path) = config.admin_tls_cert_path {
            self.admin_server.cert_path = tls_cert_path;
        }
        if let Some(tls_key_path) = config.admin_tls_key_path {
            self.admin_server.key_path = tls_key_path;
        }
        if let Some(indexer_cid_url) = config.indexer_cid_url {
            self.indexer.cid_url = indexer_cid_url;
        }
        /*
         * if let Some(indexer_mh_url) = config.indexer_mh_url {
         *     self.indexer.mh_url = indexer_mh_url;
         * }
         */
        if let Some(max_cache_size) = config.max_cache_size {
            self.cache.max_size = max_cache_size;
        }
        if let Some(ttl_buf) = config.ttl_buf {
            self.cache.ttl_buf = ttl_buf;
        }
        if let Some(ttl_cache_interval) = config.ttl_cache_interval {
            self.worker.ttl_cache_interval = ttl_cache_interval;
        }
    }
}
