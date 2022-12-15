use std::sync::Arc;

use axum::Extension;
use tokio::sync::RwLock;

use crate::{cache::LFUCacheTLL, config::GatewayConfig};

pub mod get;
pub mod post;
pub mod put;

type ExtensionLayer = Extension<(Arc<RwLock<GatewayConfig>>, Arc<RwLock<LFUCacheTLL>>)>;
