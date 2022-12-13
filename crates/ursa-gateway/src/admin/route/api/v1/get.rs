use std::sync::Arc;

use axum::{Extension, Json};
use serde_json::{json, Value};
use tokio::sync::RwLock;

use crate::config::GatewayConfig;

pub async fn get_config_handler(
    Extension(config): Extension<Arc<RwLock<GatewayConfig>>>,
) -> Json<Value> {
    Json(json!(&(*config.read().await)))
}
