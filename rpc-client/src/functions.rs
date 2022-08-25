use jsonrpc_v2::Error;

use rpc_server::{
    api::{
        NetworkGetParams, NetworkGetResult, NetworkPutCarParams, NetworkPutCarResult, NETWORK_GET,
        NETWORK_PUT_CAR,
    },
    api::{NetworkPutFileParams, NetworkPutFileResult, NETWORK_PUT_FILE},
};

use crate::{
    call,
    RpcMethod::{Post, Put},
};

pub type Result<T> = anyhow::Result<T, Error>;

pub async fn get_block(params: NetworkGetParams) -> Result<NetworkGetResult> {
    call(NETWORK_GET, params, Post).await
}

pub async fn put_car(params: NetworkPutCarParams) -> Result<NetworkPutCarResult> {
    call(NETWORK_PUT_CAR, params, Put).await
}

pub async fn put_file(params: NetworkPutFileParams) -> Result<NetworkPutFileResult> {
    call(NETWORK_PUT_FILE, params, Put).await
}
