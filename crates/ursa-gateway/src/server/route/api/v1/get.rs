use axum::{extract::Path, response::IntoResponse, Extension, Json};
use cid::Cid;
use hyper::{body, StatusCode, Uri};
use serde_json::{from_slice, json};
use std::{str::FromStr, sync::Arc};
use tracing::{debug, error};

use crate::{
    config::GatewayConfig,
    indexer::model::IndexerResponse,
    server::{model::HttpResponse, Client},
};

pub async fn get_block_handler(
    Path(cid): Path<String>,
    Extension((client, config)): Extension<(Client, Arc<GatewayConfig>)>,
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
            error!("error parsed uri: {}\n{}", endpoint, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!(HttpResponse {
                    message: Some(format!("error parsed uri: {}", endpoint)),
                    data: None
                })),
            );
        }
    };
    let resp = match client.get(uri).await {
        Ok(resp) => resp,
        Err(e) => {
            error!("error requested uri: {}\n{}", endpoint, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!(HttpResponse {
                    message: Some(format!("error requested uri: {}", endpoint)),
                    data: None
                })),
            );
        }
    };
    let bytes = match body::to_bytes(resp.into_body()).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("error read data from upstream: {}\n{}", endpoint, e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!(HttpResponse {
                    message: Some(format!("error read data from upstream: {}", endpoint)),
                    data: None
                })),
            );
        }
    };
    let indexer_response: IndexerResponse = match from_slice(&bytes) {
        Ok(indexer_response) => indexer_response,
        Err(e) => {
            error!(
                "error parsed indexer response from upstream: {}\n{}",
                endpoint, e
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!(HttpResponse {
                    message: Some(format!(
                        "error parsed indexer response from upstream: {}",
                        endpoint
                    )),
                    data: None
                })),
            );
        }
    };
    debug!(
        "received indexer response for {cid}:\n{:?}",
        indexer_response
    );
    (StatusCode::OK, Json(json!(indexer_response)))
}
