use crate::events::{track, MetricEvent};
use axum::{extract::MatchedPath, http::Request, middleware::Next, response::IntoResponse};
use metrics::Label;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::time::Instant;

pub fn setup_metrics_handler() -> PrometheusHandle {
    PrometheusBuilder::new().install_recorder().unwrap()
}

pub async fn track_metrics<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    let start = Instant::now();
    let path = if let Some(matched_path) = req.extensions().get::<MatchedPath>() {
        matched_path.as_str().to_owned()
    } else {
        req.uri().path().to_owned()
    };
    let method = req.method().clone();
    let response = next.run(req).await;

    let latency = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    let labels = vec![
        Label::new("method", method.to_string()),
        Label::new("path", path),
        Label::new("status", status),
        Label::new("latency", format!("{}", latency)),
    ];

    track(MetricEvent::RpcRequestReceived, None, None);
    track(MetricEvent::RpcResponseSent, Some(labels), Some(latency));

    response
}
