use anyhow::Result;
use jsonrpc_v2::{Error, Id, RequestObject, V2};

use serde::{Deserialize, Serialize};
use tracing::error;

/// Error object in a response
#[derive(Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum JsonRpcResponse<R> {
    Result {
        jsonrpc: V2,
        result: R,
        id: Id,
    },
    Error {
        jsonrpc: V2,
        error: JsonRpcError,
        id: Id,
    },
}

pub enum RpcMethod {
    Put,
    Post,
}

pub(crate) fn create_request<P: Serialize>(
    method_name: &str,
    params: P,
) -> Result<RequestObject, Error> {
    match serde_json::to_value(params) {
        Ok(value) => Ok(RequestObject::request()
            .with_method(method_name)
            .with_params(value)
            .with_id(1)
            .finish()),
        Err(_) => {
            error!(
            "[RPCClient] - There was an error while converting the params to serializable value"
        );
            Err(Error::Full {
                data: None,
                code: 200,
                message: "There was an error while converting the params to serializable value"
                    .to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_logger::SimpleLogger;
    use tracing::log::LevelFilter;

    use crate::api::{NetworkGetParams, NetworkPutFileParams};
    use crate::client::Client;
    use cid::{multihash::Code, Cid};
    use libipld::block::Block;
    use libipld::cbor::DagCborCodec;
    use libipld::ipld::Ipld;
    use libipld::store::DefaultParams;
    use tracing::info;

    fn create_block(ipld: Ipld) -> Block<DefaultParams> {
        Block::<DefaultParams>::encode(DagCborCodec, Code::Blake3_256, &ipld).unwrap()
    }

    fn setup_logger(level: LevelFilter) {
        SimpleLogger::new()
            .with_level(level)
            .with_utc_timestamps()
            .init()
            .unwrap();
    }

    #[ignore]
    #[tokio::test]
    async fn test_rpc_get_cid() {
        setup_logger(LevelFilter::Info);
        let block = create_block(Ipld::String("Hello World!".to_string()));
        let cid = block.cid();
        let string_cid = Cid::to_string(&cid);
        let params = NetworkGetParams {
            cid: string_cid.clone(),
        };
        let client = Client::default();
        match client.get_block(params).await {
            Ok(v) => {
                info!("Got the bytes ({v:?}) for cid({string_cid:?}) from server.");
            }
            Err(_e) => {
                error!("There was an error while calling the server. Please Check Server Logs")
            }
        };
    }

    #[ignore]
    #[tokio::test]
    async fn test_rpc_put_file() {
        setup_logger(LevelFilter::Info);
        let params = NetworkPutFileParams {
            path: "./car_files/ursa_major.car".to_string(),
        };
        let client = Client::default();
        match client.put_file(params).await {
            Ok(v) => {
                println!("Put car file done: {v:?}");
            }
            Err(_e) => {
                println!("There was an error while calling the server. Please Check Server Logs")
            }
        };
    }
}
