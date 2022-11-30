use crate::config::MetricsServiceConfig;
use anyhow::Result;
use axum::{http::StatusCode, routing::get, Router};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::future::ready;
use std::net::SocketAddr;
use tracing::info;

async fn get_ping_handler() -> (StatusCode, String) {
    (StatusCode::OK, "pong".to_string())
}

fn setup_metrics_handler(agent: String) -> PrometheusHandle {
    PrometheusBuilder::new()
        .add_global_label("agent", agent)
        .install_recorder()
        .unwrap()
}

pub async fn start(conf: &MetricsServiceConfig) -> Result<()> {
    crate::events::describe();
    let prometheus_handler = setup_metrics_handler(conf.agent.clone());

    let router = Router::new()
        .route(
            conf.api_path.as_str(),
            get(move || ready(prometheus_handler.render())),
        )
        .route("/ping", get(get_ping_handler));

    let http_address = SocketAddr::from(([0, 0, 0, 0], conf.port));
    info!("listening on {}", http_address);
    axum::Server::bind(&http_address)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}
