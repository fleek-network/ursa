use axum::{
    middleware,
    routing::{post, put},
    Router,
};
use cid::Cid;
use futures::{channel::mpsc::unbounded, SinkExt};
use std::io::Cursor;
use std::{str::FromStr, sync::Arc};
use ursa_metrics::middleware::track_metrics;

use jsonrpc_v2::{Data, Error, Params};

use crate::{
    api::{
        NetworkGetFileParams, NetworkGetParams, NetworkGetResult, NetworkInterface,
        NetworkPutFileParams, NetworkPutFileResult,
    },
    rpc::rpc::rpc_handler,
};
use tokio::sync::{mpsc::channel, RwLock};

use tracing::error;
pub type Result<T> = anyhow::Result<T, Error>;

pub fn init() -> Router {
    Router::new()
        .route("/rpc/v0", put(rpc_handler))
        .route("/rpc/v0", post(rpc_handler))
        .route_layer(middleware::from_fn(track_metrics))
}

pub async fn get_cid_handler<I>(
    data: Data<Arc<I>>,
    Params(params): Params<NetworkGetParams>,
) -> Result<NetworkGetResult>
where
    I: NetworkInterface,
{
    if let Ok(cid) = Cid::from_str(&params.cid) {
        match data.0.get(cid).await {
            Err(err) => Err(Error::internal(err)),
            Ok(res) => Ok(res.unwrap()),
        }
    } else {
        error!("Invalid Cid String, Cannot Parse {} to CID", &params.cid);
        return Err(Error::INVALID_PARAMS);
    }
}
pub async fn get_file_handler<I>(
    data: Data<Arc<I>>,
    Params(params): Params<NetworkGetFileParams>,
) -> Result<()>
where
    I: NetworkInterface,
{
    let path = params.path;
    if let Ok(cid) = Cid::from_str(&params.cid) {
        match data.0.get_file(path, cid).await {
            Err(err) => {
                error!("{:?}", err);
                return Err(Error::internal(err));
            }
            _ => Ok(()),
        }
    } else {
        error!("Invalid Cid String, Cannot Parse {} to CID", &params.cid);
        return Err(Error::INVALID_PARAMS);
    }
}

pub async fn put_file_handler<I>(
    data: Data<Arc<I>>,
    Params(params): Params<NetworkPutFileParams>,
) -> Result<NetworkPutFileResult>
where
    I: NetworkInterface,
{
    let path = params.path;

    match data.0.put_file(path).await {
        Err(err) => {
            error!("{:?}", err);
            return Err(Error::internal(err));
        }
        Ok(res) => Ok(res.iter().map(|c| Cid::from(c).to_string()).collect()),
    }
}
