use anyhow::anyhow;
use async_std::channel::bounded;
use async_std::io::Cursor;
use async_std::sync::RwLock;
use axum::{
    middleware,
    routing::{get, post, put},
    Router,
};
use cid::Cid;
use service_metrics::middleware::{setup_metrics_handler, track_metrics};
use std::{str::FromStr, sync::Arc};

use jsonrpc_v2::{Data, Error, Params};

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
use fvm_ipld_car::CarHeader;
use std::future::ready;

use tracing::error;
pub type Result<T> = anyhow::Result<T, Error>;

pub fn init() -> Router {
    Router::new()
        .route("/rpc/v0", get(http_handler))
        .route("/rpc/v0", put(http_handler))
        .route("/rpc/v0", post(http_handler))
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
    let cid = Cid::from_str(&params.cid).unwrap();
    let buf = params.data;

    let buffer: Arc<RwLock<Vec<u8>>> = Default::default();
    let header = CarHeader {
        roots: vec![cid],
        version: 1,
    };

    let (tx, mut rx) = bounded(10);

    let buffer_cloned = buffer.clone();
    let write_task = async_std::task::spawn(async move {
        header
            .write_stream_async(&mut *buffer_cloned.write().await, &mut rx)
            .await
            .unwrap()
    });

    tx.send((cid, buf)).await.unwrap();
    drop(tx);
    write_task.await;

    let buffer: Vec<_> = buffer.read().await.clone();
    if let Err(err) = data.0.put_car(Cursor::new(&buffer)).await {
        error!("{:?}", err);
    }

    Ok(true)
}

pub async fn put_file_handler<I>(
    data: Data<Arc<I>>,
    Params(params): Params<NetworkPutFileParams>,
) -> Result<NetworkPutFileResult>
where
    I: NetworkInterface,
{
    let path = params.path;

    if let Err(err) = data.0.put_file(path).await {
        error!("{:?}", err);
    }

    Ok(true)
}
