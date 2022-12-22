use std::{str::FromStr, sync::Arc};

use axum::{extract::Path, response::IntoResponse, Extension, Json};
use cid::Cid;
use hyper::{body, StatusCode, Uri};
use serde_json::{from_slice, json, Value};
use tokio::sync::RwLock;
use tracing::{debug, error};

use super::Client;
use crate::{
    cache::Tlrfu,
    config::{GatewayConfig, IndexerConfig},
    indexer::model::IndexerResponse,
    server::model::HttpResponse,
};

pub async fn get_block_handler(
    Path(cid): Path<String>,
    Extension(cache): Extension<Arc<RwLock<Tlrfu>>>,
    Extension(config): Extension<Arc<RwLock<GatewayConfig>>>,
    Extension(client): Extension<Arc<Client>>,
) -> impl IntoResponse {
    let GatewayConfig {
        indexer: IndexerConfig { cid_url },
        ..
    } = &(*config.read().await);

    if Cid::from_str(&cid).is_err() {
        return error_handler(
            StatusCode::BAD_REQUEST,
            format!("invalid cid string, cannot parse {cid} to CID"),
        );
    };

    if let Ok(Some(bytes)) = cache.write().await.get(&cid.to_string()).await {
        return (StatusCode::OK, Json(json!(&bytes)));
    }

    let endpoint = format!("{cid_url}/{cid}");
    let uri = match endpoint.parse::<Uri>() {
        Ok(uri) => uri,
        Err(e) => {
            error!("error parsed uri: {}\n{}", endpoint, e);
            return error_handler(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error parsed uri: {endpoint}"),
            );
        }
    };

    let resp = match client.get(uri).await {
        Ok(resp) => resp,
        Err(e) => {
            error!("error requested uri: {}\n{}", endpoint, e);
            return error_handler(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error requested uri: {endpoint}"),
            );
        }
    };

    let bytes = match body::to_bytes(resp.into_body()).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("error read data from upstream: {}\n{}", endpoint, e);
            return error_handler(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error read data from upstream: {endpoint}"),
            );
        }
    };

    let indexer_response: IndexerResponse = match from_slice(&bytes) {
        Ok(indexer_response) => indexer_response,
        Err(e) => {
            error!("error parsed indexer response from upstream: {endpoint}\n{e}");
            return error_handler(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error parsed indexer response from upstream: {endpoint}"),
            );
        }
    };

    debug!("received indexer response for {cid}:\n{indexer_response:?}");

    // TODO:
    // 1. filter FleekNetwork metadata
    // 2. pick node (round-robin)
    // 3. call get_block to node
    // 4.
    //   4.1 return block?
    //   4.2 resolve?
    //
    // IMPROVEMENTS:
    // 1. maintain N workers keep track of indexing data
    // 2. cherry-pick closest node
    // 3. cache TTL
    (StatusCode::OK, Json(json!(indexer_response)))
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
