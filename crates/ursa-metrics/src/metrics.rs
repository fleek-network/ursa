use anyhow::Result;
use axum::{
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    routing::get,
    Router,
};
use std::future::ready;
use std::net::SocketAddr;
use tracing::info;

use crate::{config::MetricsServiceConfig, middleware::setup_metrics_handler};

pub async fn start(conf: &MetricsServiceConfig) -> Result<()> {
    let prometheus_handler = setup_metrics_handler();

    let router = Router::new().route("/ping", get(get_ping_handler)).route(
        conf.api_path.as_str(),
        get(move || ready(prometheus_handler.render())),
    );

    let http_address = SocketAddr::from(([0, 0, 0, 0], conf.port.parse::<u16>().unwrap()));
    info!("listening on {}", http_address);
    axum::Server::bind(&http_address)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

pub async fn get_ping_handler() -> (StatusCode, String) {
    (StatusCode::OK, "pong".to_string())
}

pub async fn get_metrics_handler() -> (StatusCode, String) {
    (StatusCode::OK, "/metrics handler".to_string())
}
