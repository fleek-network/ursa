pub mod get;
pub mod put;

use axum::Json;
use hyper::StatusCode;
use jsonrpc_v2::Error::{Full, Provided};
use serde_json::{json, Value};

fn handle_error(err: jsonrpc_v2::Error) -> (StatusCode, Json<Value>) {
    match err {
        Full { code: 200, .. } => (StatusCode::OK, Json(json!(err))),
        Provided { code: 200, .. } => (StatusCode::OK, Json(json!(err))),
        Full { .. } => (StatusCode::BAD_REQUEST, Json(json!(err))),
        Provided { .. } => (StatusCode::BAD_REQUEST, Json(json!(err))),
    }
}
