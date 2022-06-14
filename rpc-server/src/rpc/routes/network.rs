use axum::{
    routing::{get, post},
    Router,
};
use jsonrpc_v2::{Data, Error, Params};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tiny_cid::Cid;

use crate::rpc::{
    api::{
        NetworkGetParams, NetworkGetResult, NetworkInterface, NetworkPutParams, NetworkPutResult,
        Result,
    },
    rpc::http_handler,
};

pub fn init() -> Router {
    Router::new()
        .route("/rpc/v0", get(http_handler))
        .route("/rpc/v0", post(http_handler))
}

pub async fn get_cid_handler<I, T>(
    data: Data<Arc<I>>,
    Params(params): Params<NetworkGetParams>,
) -> Result<NetworkGetResult>
where
    I: NetworkInterface<T>,
    T: Serialize,
{
    // let cid = params.cid;
    // data.0.get(cid);

    Ok(true)
}

pub async fn put_car_handler<I, T>(
    data: Data<Arc<I>>,
    Params(params): Params<NetworkPutParams>,
) -> Result<NetworkPutResult>
where
    I: NetworkInterface<T>,
    T: Serialize,
{
    // let cid = params.cid;
    // data.0.get(cid);

    Ok(true)
}
