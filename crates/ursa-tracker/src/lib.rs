pub use crate::types::{Client, NodeAnnouncement};
use anyhow::{anyhow, Result};
use hyper::{Method, Request};
use serde_json::json;

pub mod types;

pub async fn register_with_tracker(
    tracker: String,
    announcement: NodeAnnouncement,
) -> Result<String> {
    let client = Client::new();

    let req = Request::builder()
        .method(Method::POST)
        .uri(tracker)
        .header("Content-Type", "application/json")
        .body(json!(announcement).to_string().into())?;

    let res = client.request(req).await.map_err(|e| anyhow!(e))?;
    let status = res.status();
    let body = String::from_utf8(hyper::body::to_bytes(res).await?.to_vec())?;
    if status.is_success() {
        Ok(body)
    } else {
        Err(anyhow!("Tracker returned error: {} - {}", status, body))
    }
}
