use axum::{extract::MatchedPath, http::Request, middleware::Next, response::IntoResponse};
use metrics::{histogram, increment_counter, Label};
use std::time::Instant;

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

    increment_counter!("rpc_request_received");
    if latency != 0.0 {
        histogram!(
            "rpc_response_duration",
            latency,
            vec![
                Label::new("method", method.to_string()),
                Label::new("path", path),
                Label::new("status", status),
            ]
        );
    }

    response
}
