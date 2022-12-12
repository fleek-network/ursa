use axum::{extract::Path, response::IntoResponse, Extension, Json};
use cid::Cid;
use hyper::{StatusCode, Uri};
use serde_json::json;
use std::str::FromStr;
use tracing::debug;

use crate::{
    config::GatewayConfig,
    server::{model::HttpResponse, Client},
};

pub async fn get_block_handler(
    Path(cid): Path<String>,
    Extension((client, config)): Extension<(Client, GatewayConfig)>,
) -> impl IntoResponse {
    if Cid::from_str(&cid).is_err() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!(HttpResponse {
                message: Some(format!("invalid cid string, cannot parse {} to CID", cid)),
                data: None
            })),
        );
    };
    let endpoint = format!("{}/{}", config.indexer.cid_url, cid);
    let uri = match endpoint.parse::<Uri>() {
        Ok(uri) => uri,
        Err(e) => {
            debug!("error parsed uri: {}\n{}", endpoint, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!(HttpResponse {
                    message: Some(format!("error parsed uri: {}", endpoint)),
                    data: None
                })),
            );
        }
    };
    let _resp = match client.get(uri).await {
        Ok(resp) => resp,
        Err(e) => {
            debug!("error requested uri: {}\n{}", endpoint, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!(HttpResponse {
                    message: Some(format!("error requested uri: {}", endpoint)),
                    data: None
                })),
            );
        }
    };
    (StatusCode::OK, Json(json!("")))
}
