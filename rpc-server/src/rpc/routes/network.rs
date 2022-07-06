use async_std::io::Cursor;
use axum::{
    routing::{get, post},
    Router,
};
use jsonrpc_v2::{Data, Error, Params};
use std::sync::Arc;

use crate::{
    api::{NetworkPutFileParams, NetworkPutFileResult},
    rpc::{
        api::{
            NetworkGetParams, NetworkGetResult, NetworkInterface, NetworkPutCarParams,
            NetworkPutCarResult,
        },
        rpc::http_handler,
    },
};

pub type Result<T> = anyhow::Result<T, Error>;

pub fn init() -> Router {
    Router::new()
        .route("/rpc/v0", get(http_handler))
        .route("/rpc/v0", post(http_handler))
}

pub async fn get_cid_handler<I>(
    data: Data<Arc<I>>,
    Params(params): Params<NetworkGetParams>,
) -> Result<NetworkGetResult>
where
    I: NetworkInterface,
{
    let _ = data.0.get(params.cid).await;

    Ok(true)
}

pub async fn put_car_handler<I>(
    data: Data<Arc<I>>,
    Params(params): Params<NetworkPutCarParams>,
) -> Result<NetworkPutCarResult>
where
    I: NetworkInterface,
{
    let cid = params.cid;
    let buffer = params.data;

    let _ = data.0.put_car(cid, Cursor::new(&buffer)).await;

    Ok(true)
}

pub async fn put_file_handler<I>(
    data: Data<Arc<I>>,
    Params(params): Params<NetworkPutFileParams>,
) -> Result<NetworkPutFileResult>
where
    I: NetworkInterface,
{
    let cid = params.cid;
    let path = params.path;

    let _ = data.0.put_file(cid, path).await;

    Ok(true)
}
