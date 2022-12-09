use axum::{extract::Path, response::IntoResponse};
use hyper::StatusCode;

pub async fn get_block_handler(Path(cid): Path<String>) -> impl IntoResponse {
    (StatusCode::OK, cid)
}
