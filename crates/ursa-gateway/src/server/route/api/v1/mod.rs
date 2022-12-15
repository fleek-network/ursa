use std::sync::Arc;

use axum::Extension;
use hyper::{client::HttpConnector, Body};
use hyper_tls::HttpsConnector;
use tokio::sync::RwLock;

use crate::{cache::LFUCacheTLL, config::GatewayConfig};

pub mod get;
pub mod put;

type Client = hyper::client::Client<HttpsConnector<HttpConnector>, Body>;

type ExtensionLayer = Extension<(
    Arc<Client>,
    Arc<RwLock<GatewayConfig>>,
    Arc<RwLock<LFUCacheTLL>>,
)>;
