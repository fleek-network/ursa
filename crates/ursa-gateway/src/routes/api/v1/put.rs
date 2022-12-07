use axum::{response::IntoResponse, Json};
use hyper::StatusCode;
use serde_json::json;
use ursa_rpc_client::functions::put_file;
use ursa_rpc_server::api::NetworkPutFileParams;

pub async fn put_file_handler(Json(params): Json<NetworkPutFileParams>) -> impl IntoResponse {
    match put_file(params).await {
        Ok(res) => (StatusCode::OK, Json(json!(res))),
        Err(err) => super::handle_error(err),
    }
}
