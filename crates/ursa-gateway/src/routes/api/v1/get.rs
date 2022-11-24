use axum::{extract::Query, response::IntoResponse, Json};
use hyper::StatusCode;
use serde_json::json;
use ursa_rpc_client::functions::get_block;
use ursa_rpc_server::api::NetworkGetParams;

pub async fn get_block_handler(Query(params): Query<NetworkGetParams>) -> impl IntoResponse {
    match get_block(params).await {
        Ok(res) => (StatusCode::OK, Json(json!(res))),
        Err(err) => super::handle_error(err),
    }
}
