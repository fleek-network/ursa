use anyhow::anyhow;
use async_std::io::Cursor;
use axum::{
    middleware,
    routing::{get, post, put},
    Router,
};
use cid::Cid;
use std::{str::FromStr, sync::Arc};

use jsonrpc_v2::{Data, Error, Params};

use crate::rpc::routes::metrics::{setup_metrics_handler, track_metrics};
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
use std::future::ready;

use tracing::{error, info, warn};
pub type Result<T> = anyhow::Result<T, Error>;

pub fn init() -> Router {
    let metrics_handler = setup_metrics_handler();

    Router::new()
        .route("/rpc/v0", get(http_handler))
        .route("/rpc/v0", put(http_handler))
        .route("/rpc/v0", post(http_handler))
        .route("/metrics", get(move || ready(metrics_handler.render())))
        .route_layer(middleware::from_fn(track_metrics))
}

pub async fn get_cid_handler<I>(
    data: Data<Arc<I>>,
    Params(params): Params<NetworkGetParams>,
) -> Result<NetworkGetResult>
where
    I: NetworkInterface,
{
    let cid = Cid::from_str(&params.cid).unwrap();
    match data.0.get(cid).await {
        Err(_err) => Err(Error::Full {
            data: None,
            code: 200,
            message: "There was an error while getting the block".to_string(),
        }),
        Ok(res) => Ok(res),
    }
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
    let cid = Cid::from_str(&params.cid).unwrap();
    let path = params.path;

    if let Err(err) = data.0.put_file(cid, path).await {
        error!("{:?}", err);
    }

    Ok(true)
}
