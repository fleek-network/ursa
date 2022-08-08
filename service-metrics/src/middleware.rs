use crate::events;
use metrics::Label;
use axum::{extract::MatchedPath, http::Request, middleware::Next, response::IntoResponse};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::time::Instant;

const REQUEST_DURATION_LABEL: &'static str = "http_requests_duration_seconds";

pub fn setup_metrics_handler() -> PrometheusHandle {
    PrometheusBuilder::new()
        //.set_buckets_for_metric(
        //Matcher::Full(REQUEST_DURATION_LABEL.to_string()),
        //&[0.005, 0.01, 0.025, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0],
        //)
        //.unwrap()
        .install_recorder()
        .unwrap()
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

    events::track(events::RPC_REQUEST_RECEIVED, None, None);
    events::track(events::RPC_RESPONSE_SENT, Some(labels), Some(latency));

    response
}
