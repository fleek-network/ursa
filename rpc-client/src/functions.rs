use jsonrpc_v2::Error;

use rpc_server::rpc::api::{NetworkGetParams, NetworkGetResult, NETWORK_GET};

use crate::{
    call,
    HttpMethod::{Get, Put},
};

pub type Result<T> = anyhow::Result<T, Error>;

pub async fn get_block(params: NetworkGetParams) -> Result<NetworkGetResult> {
    call(NETWORK_GET, params, Get).await
}
