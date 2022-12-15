use axum::{Extension, Json};
use serde_json::{json, Value};

pub async fn get_config_handler(Extension((config, _)): super::ExtensionLayer) -> Json<Value> {
    Json(json!(&(*config.read().await)))
}

pub async fn get_cache_handler(Extension((_, cache)): super::ExtensionLayer) -> Json<Value> {
    Json(json!(&(*cache.read().await)))
}
