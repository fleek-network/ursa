use crate::config::MetricsServiceConfig;
use anyhow::Result;
use axum::{http::StatusCode, routing::get, Router, Extension};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;
use prometheus::{TextEncoder, Encoder};

async fn get_ping_handler() -> (StatusCode, String) {
    (StatusCode::OK, "pong".to_string())
}

fn setup_metrics_handler(agent: String) -> PrometheusHandle {
    PrometheusBuilder::new()
        .add_global_label("agent", agent)
        .install_recorder()
        .unwrap()
}

async fn metrics_handler(
    handle: Extension<Arc<PrometheusHandle>>,
) -> (StatusCode, String) {
    // ursa metrics
    let mut metrics = handle.render();

    // Collect metrics provided from bitswap and append them to the metrics string
    let mut buffer = Vec::new();
    TextEncoder::new().encode(&prometheus::gather(), &mut buffer).unwrap();
    if buffer.len() > 0 {
        metrics.push_str(&String::from_utf8(buffer).unwrap());
    }

    (StatusCode::OK, metrics)
}

pub async fn start(conf: &MetricsServiceConfig) -> Result<()> {
    let prometheus_handler = setup_metrics_handler(conf.agent.clone());

    let router = Router::new()
        .route(
            "/metrics",
            get(metrics_handler),
        )
        .route("/ping", get(get_ping_handler))
        .layer(Extension(Arc::new(prometheus_handler)));

    let http_address = SocketAddr::from(([0, 0, 0, 0], conf.port));
    info!("listening on {}", http_address);
    axum::Server::bind(&http_address)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}
