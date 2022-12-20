use crate::BITSWAP_REGISTRY;
use axum::{http::StatusCode, routing::get, Extension, Router};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use prometheus::{Encoder, TextEncoder};
use std::sync::Arc;

async fn metrics_handler(handle: Extension<Arc<PrometheusHandle>>) -> (StatusCode, String) {
    // ursa metrics
    let mut metrics = handle.render();

    // Collect metrics provided from bitswap and append them to the metrics string
    let mut buffer = Vec::new();
    TextEncoder::new()
        .encode(&BITSWAP_REGISTRY.gather(), &mut buffer)
        .unwrap();
    if !buffer.is_empty() {
        metrics.push_str(&String::from_utf8(buffer).unwrap());
    }

    (StatusCode::OK, metrics)
}

pub fn init() -> Router {
    Router::new()
        .route("/metrics", get(metrics_handler))
        .layer(Extension(Arc::new(
            PrometheusBuilder::new().install_recorder().unwrap(),
        )))
}
