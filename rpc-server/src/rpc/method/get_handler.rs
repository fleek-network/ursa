use jsonrpc_v2::{Data, Error, Params};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tiny_cid::Cid;

use crate::rpc::api::NetworkInterface;

pub type Result<T> = anyhow::Result<T, Error>;

#[derive(Deserialize)]
pub struct GetHandlerParams {
    pub cid: Cid,
}

pub async fn get_handler<I, T>(
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
