use std::sync::Arc;

use axum::Extension;
use hyper::StatusCode;
use tokio::sync::RwLock;

use crate::cache::Tlrfu;

pub async fn purge_cache_handler(Extension(cache): Extension<Arc<RwLock<Tlrfu>>>) -> StatusCode {
    cache.write().await.purge();
    StatusCode::OK
}
