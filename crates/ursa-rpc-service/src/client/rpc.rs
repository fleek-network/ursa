use anyhow::Result;
use jsonrpc_v2::{Error, Id, RequestObject, V2};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

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

/// Utility method for sending RPC requests over HTTP
pub async fn call<P, R>(
    method_name: &str,
    params: P,
    method: RpcMethod,
    rpc_port: Option<u16>,
) -> Result<R, Error>
where
    P: Serialize,
    R: DeserializeOwned,
{
    if let Ok(value) = serde_json::to_value(params) {
        let rpc_req = RequestObject::request()
            .with_method(method_name)
            .with_params(value)
            .with_id(1)
            .finish();

        let port = if let Some(rpc_port) = rpc_port {
            rpc_port
        } else {
            4069
        };
        let addr = "0.0.0.0".to_string();
        let api_url = format!("http://{addr}:{port}/rpc/v0");

        info!("Using JSON-RPC v2 HTTP URL: {api_url}");
        debug!("rpc_req {:?}", rpc_req);

        // TODO(arslan): Add authentication
        if let Ok(from_json) = surf::Body::from_json(&rpc_req) {
            let mut http_res = match method {
                RpcMethod::Post => surf::post(api_url)
                    .content_type("application/json")
                    .body(from_json)
                    .await
                    .unwrap(),
                RpcMethod::Put => surf::put(api_url)
                    .content_type("application/json")
                    .body(from_json)
                    .await
                    .unwrap(),
            };
            let res = http_res.body_string().await.unwrap();

            let code = http_res.status() as i64;

            if code != 200 {
                error!("[RPCClient] - server responded with http error code {code:?} - {res}");
                return Err(Error::Full {
                    message: format!("Error code from HTTP Response: {code}"),
                    code,
                    data: None,
                });
            }

            // Return the parsed RPC result
            let rpc_res: JsonRpcResponse<R> = match serde_json::from_str(&res) {
                Ok(r) => r,
                Err(e) => {
                    return Err(Error::Full {
                        data: None,
                        code: 200,
                        message: format!("Parse Error: {e}\nData: {res}"),
                    })
                }
            };

            match rpc_res {
                JsonRpcResponse::Result { result, .. } => Ok(result),
                JsonRpcResponse::Error { error, .. } => Err(Error::Full {
                    data: None,
                    code: error.code,
                    message: error.message,
                }),
            }
        } else {
            error!("[RPCClient] - There was an while serializing the rpc request");
            Err(Error::Full {
                data: None,
                code: 200,
                message: "There was an while serializing the rpc request".to_string(),
            })
        }
    } else {
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

#[cfg(test)]
mod tests {
    use super::*;
    use simple_logger::SimpleLogger;
    use tracing::log::LevelFilter;

    use crate::client::functions::{get_block, put_file};

    use crate::api::{NetworkGetParams, NetworkPutFileParams};
    use cid::{multihash::Code, Cid};
    use libipld::block::Block;
    use libipld::cbor::DagCborCodec;
    use libipld::ipld::Ipld;
    use libipld::store::DefaultParams;
    use ursa_utils::convert_cid;

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

    #[tokio::test]
    async fn test_rpc_get_cid() {
        setup_logger(LevelFilter::Info);
        let block = create_block(Ipld::String("Hello World!".to_string()));
        let cid = convert_cid(block.cid().to_bytes());
        let string_cid = Cid::to_string(&cid);
        let params = NetworkGetParams {
            cid: string_cid.clone(),
        };
        match get_block(params).await {
            Ok(v) => {
                info!("Got the bytes ({v:?}) for cid({string_cid:?}) from server.");
            }
            Err(_e) => {
                error!("There was an error while calling the server. Please Check Server Logs")
            }
        };
    }

    #[tokio::test]
    async fn test_rpc_put_file() {
        setup_logger(LevelFilter::Info);
        let params = NetworkPutFileParams {
            path: "./car_files/ursa_major.car".to_string(),
        };
        match put_file(params, None).await {
            Ok(v) => {
                println!("Put car file done: {v:?}");
            }
            Err(_e) => {
                println!("There was an error while calling the server. Please Check Server Logs")
            }
        };
    }
}
