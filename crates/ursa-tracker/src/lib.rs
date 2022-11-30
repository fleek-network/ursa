use crate::types::{Client, NodeAnnouncement};
use anyhow::{anyhow, Result};
use hyper::{Method, Request};
use serde_json::json;

pub mod types;

pub async fn track_node(tracker: String, announcement: NodeAnnouncement) -> Result<String> {
    let client = Client::new();

    let req = Request::builder()
        .method(Method::POST)
        .uri(tracker)
        .header("Content-Type", "application/json")
        .body(json!(announcement).to_string().into())?;

    let res = client.request(req).await.map_err(|e| anyhow!(e))?;
    if res.status().is_success() {
        let body = hyper::body::to_bytes(res).await?;
        Ok(String::from_utf8(body.to_vec())?)
    } else {
        Err(anyhow!("Tracker returned error: {}", res.status()))
    }
}
