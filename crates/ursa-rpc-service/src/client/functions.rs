use jsonrpc_v2::Error;

use crate::api::{
    NetworkGetFileParams, NetworkGetParams, NetworkGetResult, NetworkPutFileParams,
    NetworkPutFileResult, NETWORK_GET, NETWORK_GET_FILE, NETWORK_PUT_FILE,
};

use super::{
    call,
    RpcMethod::{Post, Put},
};

pub type Result<T> = anyhow::Result<T, Error>;

pub async fn get_block(params: NetworkGetParams) -> Result<NetworkGetResult> {
    call(NETWORK_GET, params, Post, None).await
}

pub async fn get_file(params: NetworkGetFileParams, rpc_port: Option<u16>) -> Result<()> {
    call(NETWORK_GET_FILE, params, Put, rpc_port).await
}

pub async fn put_file(
    params: NetworkPutFileParams,
    rpc_port: Option<u16>,
) -> Result<NetworkPutFileResult> {
    call(NETWORK_PUT_FILE, params, Put, rpc_port).await
}