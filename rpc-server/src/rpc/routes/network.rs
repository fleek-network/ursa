use axum::{
    routing::{get, post},
    Router,
};
use futures::AsyncRead;
use jsonrpc_v2::{Data, Error, Params};
use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::rpc::{
    api::{
        NetworkGetParams, NetworkGetResult, NetworkInterface, NetworkPutCarParams,
        NetworkPutCarResult,
    },
    rpc::http_handler,
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
    let cid = params.cid;
    data.0.get(cid);

    Ok(true)
}

pub async fn put_car_handler<I, R: AsyncRead + Send + Unpin>(
    data: Data<Arc<I>>,
    Params(params): Params<NetworkPutCarParams<R>>,
) -> Result<NetworkPutCarResult>
where
    I: NetworkInterface,
{
    let cid = params.cid;
    let reader = params.reader;

    data.0.put_car(cid, reader);

    Ok(true)
}
