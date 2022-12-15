use axum::Extension;
use hyper::StatusCode;

pub async fn purge_cache_handler(Extension((_, cache)): super::ExtensionLayer) -> StatusCode {
    cache.write().await.purge();
    StatusCode::OK
}
