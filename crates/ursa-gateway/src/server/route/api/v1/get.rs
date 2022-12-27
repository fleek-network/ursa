use std::{str::FromStr, sync::Arc};

use axum::{extract::Path, response::IntoResponse, Extension, Json};
use cid::Cid;
use hyper::{body, Body, Request, StatusCode, Uri};
use serde_json::{from_slice, json, Value};
use tokio::sync::RwLock;
use tracing::{debug, error};
use ursa_rpc_service::api::NetworkGetResult;
use ursa_rpc_service::client::JsonRpcResponse;

use super::Client;
use crate::indexer::choose_provider;
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

    let provider_resp = match choose_provider(indexer_response) {
        Ok(addrs) => {
            let mut addr_iter = addrs.into_iter();

            loop {
                match addr_iter.next() {
                    Some(addr) => {
                        let req = Request::builder()
                            .method("POST")
                            .uri(format!("http://{}:{}/rpc/v0", addr.ip(), addr.port()))
                            .header("Content-Type", "application/json")
                            .body(Body::from(
                                json!({
                                    "jsonrpc": "2.0",
                                    "id": "id",
                                    "method": "ursa_get_cid",
                                    "params":{"cid": cid}
                                })
                                .to_string(),
                            ))
                            .expect("Request to be valid.");

                        match client.request(req).await {
                            Ok(resp) if resp.status() == 404 => {
                                return error_handler(
                                    StatusCode::NOT_FOUND,
                                    format!("failed to find content for {cid}"),
                                )
                            }
                            Ok(resp) => break resp,
                            Err(e) => error!("querying the node provider failed {e}"),
                        }
                    }
                    None => {
                        return error_handler(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("failed to get data"),
                        )
                    }
                }
            }
        }
        Err(e) => {
            error!("failed to choose provider: {e}");
            return error_handler(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to choose provider"),
            );
        }
    };

    let block_bytes = match body::to_bytes(provider_resp.into_body()).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("failed to read data from upstream: {}\n{}", endpoint, e);
            return error_handler(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to read data from upstream: {endpoint}"),
            );
        }
    };

    let block = match from_slice::<JsonRpcResponse<NetworkGetResult>>(&block_bytes) {
        Ok(JsonRpcResponse::Result { result, .. }) => result,
        Ok(JsonRpcResponse::Error { error, .. }) => {
            error!(
                "server returned error with code {} and message {}",
                error.code, error.message
            );
            return error_handler(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
                    "server returned error with code {} and message {}",
                    error.code, error.message
                ),
            );
        }
        Err(e) => {
            error!("error parsed response from provider: {e}");
            return error_handler(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error parsed response from provider: {e}"),
            );
        }
    };

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
    (
        StatusCode::OK,
        Json(json!(HttpResponse {
            message: None,
            data: Some(block.to_vec())
        })),
    )
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
