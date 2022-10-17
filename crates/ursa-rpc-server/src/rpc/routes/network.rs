use axum::{
    middleware,
    routing::{post, put},
    Router,
};
use cid::Cid;
use std::io::Cursor;
use std::{str::FromStr, sync::Arc};
use ursa_metrics::middleware::track_metrics;

use jsonrpc_v2::{Data, Error, Params};

use crate::{
    api::{
        NetworkGetParams, NetworkGetResult, NetworkInterface, NetworkPutCarParams,
        NetworkPutCarResult, NetworkPutFileParams, NetworkPutFileResult,
    },
    rpc::rpc::rpc_handler,
};
use fvm_ipld_car::CarHeader;
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
    let cid = Cid::from_str(&params.cid).unwrap();
    match data.0.get(cid).await {
        Err(_err) => Err(Error::Full {
            data: None,
            code: -32000,
            message: "There was an error while getting the block".to_string(),
        }),
        Ok(res) => Ok(res.unwrap()),
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

    let (tx, mut rx) = channel(10);

    let buffer_cloned = buffer.clone();
    let write_task = tokio::spawn(async move {
        header
            .write_stream_async(&mut *buffer_cloned.write().await, &mut rx)
            .await
            .unwrap()
    });

    tx.send((cid, buf)).await.unwrap();
    drop(tx);
    let _ = write_task.await;

    let buffer: Vec<_> = buffer.read().await.clone();
    match data.0.put_car(Cursor::new(&buffer)).await {
        Err(err) => {
            error!("{:?}", err);
            return Err(Error::Full {
                data: None,
                code: -32001,
                message: "There was an error in put_car".to_string(),
            });
        }
        Ok(res) => Ok(res.iter().map(|c| Cid::from(c).to_string()).collect()),
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
            return Err(Error::Full {
                data: None,
                code: -32001,
                message: "There was an error in put_file".to_string(),
            });
        }
        Ok(res) => Ok(res.iter().map(|c| Cid::from(c).to_string()).collect()),
    }
}
