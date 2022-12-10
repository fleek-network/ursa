pub use crate::types::NodeAnnouncement;
use anyhow::{anyhow, Result};
use hyper::{Client, Method, Request};
use hyper_tls::HttpsConnector;
use serde_json::json;

pub mod types;

pub async fn register_with_tracker(
    tracker: String,
    announcement: NodeAnnouncement,
) -> Result<String> {
    let client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());

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
