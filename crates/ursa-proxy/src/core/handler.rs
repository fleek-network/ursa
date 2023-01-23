use crate::core::ServerConfig;
use axum::response::{IntoResponse, Response};
use axum::Extension;
use std::sync::Arc;

pub async fn proxy_pass(Extension(config): Extension<Arc<ServerConfig>>) -> Response {
    format!("Sending request to {:?}:{:?}", config.addr, config.port).into_response()
}
