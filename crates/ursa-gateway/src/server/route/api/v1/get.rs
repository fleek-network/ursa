use std::{str::FromStr, sync::Arc};

use axum::{extract::Path, response::IntoResponse, Extension, Json};
use cid::Cid;
use hyper::StatusCode;
use serde_json::{json, Value};
use tokio::sync::RwLock;

use crate::{server::model::HttpResponse, worker::cache::ServerCache};

pub async fn get_block_handler<Cache: ServerCache>(
    Path(cid): Path<String>,
    Extension(cache): Extension<Arc<RwLock<Cache>>>,
) -> impl IntoResponse {
    if Cid::from_str(&cid).is_err() {
        return error_handler(
            StatusCode::BAD_REQUEST,
            format!("invalid cid string, cannot parse {cid} to CID"),
        );
    };

    match cache.read().await.get_announce(&cid).await {
        Ok(data) => (StatusCode::OK, Json(json!(data.as_ref()))),
        Err(message) => error_handler(StatusCode::INTERNAL_SERVER_ERROR, message.to_string()),
    }
}

fn error_handler(status_code: StatusCode, message: String) -> (StatusCode, Json<Value>) {
    (
        status_code,
        Json(json!(HttpResponse {
            message: Some(message),
            data: None
        })),
    )
}
