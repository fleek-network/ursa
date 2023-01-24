use axum::{
    middleware,
    routing::{post, put},
    Router,
};
use libipld::Cid;
use std::{str::FromStr, sync::Arc};
use ursa_metrics::middleware::track_metrics;

use jsonrpc_v2::{Data, Error, Params};

use crate::{
    api::{
        NetworkGetFileParams, NetworkGetListenerAddresses, NetworkGetParams, NetworkGetPeers,
        NetworkGetResult, NetworkInterface, NetworkPutFileParams, NetworkPutFileResult,
    },
    rpc::rpc_handler,
};
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
            Ok(res) => Ok(res),
        }
    } else {
        error!("Invalid Cid String, Cannot Parse {} to CID", &params.cid);
        Err(Error::INVALID_PARAMS)
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
                Err(Error::internal(err))
            }
            _ => Ok(()),
        }
    } else {
        error!("Invalid Cid String, Cannot Parse {} to CID", &params.cid);
        Err(Error::INVALID_PARAMS)
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
            Err(Error::internal(err))
        }
        Ok(res) => Ok(res.iter().map(|c| Cid::from(c).to_string()).collect()),
    }
}

pub async fn get_peers<I>(data: Data<Arc<I>>) -> Result<NetworkGetPeers>
where
    I: NetworkInterface,
{
    match data.0.get_peers().await {
        Err(err) => {
            error!("{:?}", err);
            Err(Error::internal(err))
        }
        Ok(res) => Ok(res),
    }
}

pub async fn get_listener_addresses<I>(data: Data<Arc<I>>) -> Result<NetworkGetListenerAddresses>
where
    I: NetworkInterface,
{
    if cfg!(test) {
        // for rpc server unit test
        Ok(Vec::from(["/ip4/127.0.0.1/tcp/6009".parse().unwrap()]))
    } else {
        match data.0.get_listener_addresses().await {
            Err(err) => {
                error!("{:?}", err);
                Err(Error::internal(err))
            }
            Ok(res) => Ok(res),
        }
    }
}
