use {
    crate::worker::cache::AdminCache,
    axum::Extension,
    hyper::StatusCode,
    std::sync::Arc,
    tokio::sync::RwLock,
};

pub async fn purge_cache_handler<Cache: AdminCache>(
    Extension(cache): Extension<Arc<RwLock<Cache>>>,
) -> StatusCode {
    cache.write().await.purge();
    StatusCode::OK
}
