use axum::{
    routing::{get, post},
    Router,
};
use jsonrpc_v2::{Data, Error, Params};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tiny_cid::Cid;

use crate::rpc::{api::NetworkInterface, rpc::handler};

pub type Result<T> = anyhow::Result<T, Error>;

#[derive(Deserialize)]
pub struct GetHandlerParams {
    pub cid: Cid,
}

#[derive(Deserialize)]
pub struct PutHandlerParams {
    pub cid: Cid,
}

pub async fn get_cid_handler<I, T>(
    data: Data<Arc<I>>,
    Params(params): Params<GetHandlerParams>,
) -> Result<()>
where
    I: NetworkInterface<T>,
    T: Serialize,
{
    // let cid = params.cid;
    // data.0.get(cid);

    Ok(())
}

pub async fn put_car_handler<I, T>(
    data: Data<Arc<I>>,
    Params(params): Params<PutHandlerParams>,
) -> Result<()>
where
    I: NetworkInterface<T>,
    T: Serialize,
{
    // let cid = params.cid;
    // data.0.get(cid);

    Ok(())
}

pub fn init() -> Router {
    Router::new()
        .route("/rpc/v0", get(handler))
        .route("/rpc/v0", post(handler))
}
