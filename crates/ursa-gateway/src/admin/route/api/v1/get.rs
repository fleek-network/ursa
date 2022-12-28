use {
    crate::config::GatewayConfig,
    axum::{Extension, Json},
    serde_json::{json, Value},
    std::sync::Arc,
    tokio::sync::RwLock,
};

pub async fn get_config_handler(
    Extension(config): Extension<Arc<RwLock<GatewayConfig>>>,
) -> Json<Value> {
    Json(json!(&(*config.read().await)))
}
