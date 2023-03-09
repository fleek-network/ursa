use jsonrpc_v2::Error;

use crate::api::{
    EthCall, NetworkGetFileParams, NetworkGetParams, NetworkGetResult, NetworkPutFileParams,
    NetworkPutFileResult, ETH_CALL, ETH_SEND_TRANSACTION, NETWORK_GET, NETWORK_GET_FILE,
    NETWORK_PUT_FILE, EthSendTransactionParams,
};

use super::{
    call,
    RpcMethod::{Post, Put},
};

pub type Result<T> = anyhow::Result<T, Error>;

pub async fn get_block(params: NetworkGetParams) -> Result<NetworkGetResult> {
    call(NETWORK_GET, params, Post).await
}

pub async fn get_file(params: NetworkGetFileParams) -> Result<()> {
    call(NETWORK_GET_FILE, params, Put).await
}

pub async fn put_file(params: NetworkPutFileParams) -> Result<NetworkPutFileResult> {
    call(NETWORK_PUT_FILE, params, Put).await
}

pub async fn eth_send_transaction(params: EthSendTransactionParams) -> Result<()> {
    call(ETH_SEND_TRANSACTION, params, Post).await
}

pub async fn eth_call(params: EthSendTransactionParams) -> Result<EthCall> {
    call(ETH_CALL, params, Put).await
}
