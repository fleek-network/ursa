use axum::{response::IntoResponse, Json};
use hyper::StatusCode;
use serde_json::json;
use ursa_rpc_client::functions::{put_car, put_file};
use ursa_rpc_server::api::{NetworkPutCarParams, NetworkPutFileParams};

pub async fn put_car_handler(Json(params): Json<NetworkPutCarParams>) -> impl IntoResponse {
    match put_car(params).await {
        Ok(res) => (StatusCode::OK, Json(json!(res))),
        Err(err) => super::handle_error(err),
    }
}

pub async fn put_file_handler(Json(params): Json<NetworkPutFileParams>) -> impl IntoResponse {
    match put_file(params).await {
        Ok(res) => (StatusCode::OK, Json(json!(res))),
        Err(err) => super::handle_error(err),
    }
}
