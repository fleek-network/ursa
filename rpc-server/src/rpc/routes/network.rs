use axum::{
    routing::{get, post},
    Router,
};
use futures::channel::oneshot;
use jsonrpc_v2::{Data, Error, Params};
use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::rpc::{
    api::{
        NetworkGetParams, NetworkGetResult, NetworkInterface, NetworkPutCarParams,
        NetworkPutCarResult, Result,
    },
    rpc::http_handler,
};

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

pub async fn put_car_handler<I>(
    data: Data<Arc<I>>,
    Params(params): Params<NetworkPutCarParams>,
) -> Result<NetworkPutCarResult>
where
    I: NetworkInterface,
{
    Ok(true)
}
