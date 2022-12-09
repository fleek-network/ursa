use crate::config::MetricsServiceConfig;
use anyhow::Result;
use axum::{http::StatusCode, routing::get, Extension, Router};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use prometheus::{Encoder, TextEncoder, Registry};
use std::net::SocketAddr;
use std::sync::Arc;
use lazy_static::lazy_static;
use tracing::info;

lazy_static!(
    pub static ref BITSWAP_REGISTRY: Arc<Registry> = Arc::new(Registry::new());
);


async fn get_ping_handler() -> (StatusCode, String) {
    (StatusCode::OK, "pong".to_string())
}

fn setup_metrics_handler() -> PrometheusHandle {
    PrometheusBuilder::new()
        .install_recorder()
        .unwrap()
}

async fn metrics_handler(handle: Extension<Arc<PrometheusHandle>>) -> (StatusCode, String) {
    // ursa metrics
    let mut metrics = handle.render();

    // Collect metrics provided from bitswap and append them to the metrics string
    let mut buffer = Vec::new();
    TextEncoder::new()
        .encode(&BITSWAP_REGISTRY.gather(), &mut buffer)
        .unwrap();
    if buffer.len() > 0 {
        metrics.push_str(&String::from_utf8(buffer).unwrap());
    }

    (StatusCode::OK, metrics)
}

pub async fn start(conf: &MetricsServiceConfig) -> Result<()> {
    let prometheus_handler = setup_metrics_handler();

    let router = Router::new()
        .route("/metrics", get(metrics_handler))
        .route("/ping", get(get_ping_handler))
        .layer(Extension(Arc::new(prometheus_handler)));

    let http_address = SocketAddr::from(([0, 0, 0, 0], conf.port));
    info!("listening on {}", http_address);
    axum::Server::bind(&http_address)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}
