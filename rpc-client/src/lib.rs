mod functions;

use anyhow::Result;
use jsonrpc_v2::{Error, Id, RequestObject, V2};

use rpc_server::config::RpcConfig;
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

pub enum HttpMethod {
    Get,
    Put,
    Post,
}

/// Utility method for sending RPC requests over HTTP
async fn call<P, R>(method_name: &str, params: P, method: HttpMethod) -> Result<R, Error>
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

        let RpcConfig { rpc_port, rpc_addr } = RpcConfig::default();
        let api_url = format!("http://{}:{}/rpc/v0", rpc_addr, rpc_port);

        info!("Using JSON-RPC v2 HTTP URL: {}", api_url);
        debug!("rpc_req {:?}", rpc_req);

        // TODO(arslan): Add authentication
        if let Ok(from_json) = surf::Body::from_json(&rpc_req) {
            let mut http_res = match method {
                HttpMethod::Get => surf::get(api_url)
                    .content_type("application/json")
                    .body(from_json)
                    .await
                    .unwrap(),
                HttpMethod::Put => surf::put(api_url)
                    .content_type("application/json")
                    .body(from_json)
                    .await
                    .unwrap(),
                HttpMethod::Post => todo!(),
            };
            let res = http_res.body_string().await.unwrap();

            let code = http_res.status() as i64;

            if code != 200 {
                error!(
                    "[RPCClient] - server responded with the error code {:?}",
                    code
                );
                return Err(Error::Full {
                    message: format!("Error code from HTTP Response: {}", code),
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
                        message: format!(
                            "Parse Error: Response from RPC endpoint could not be parsed. Error was: {}",
                            e,
                        ),
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

    use std::str::FromStr;

    use crate::functions::{get_block, put_file};

    use cid::Cid;
    use libipld::{cbor::DagCborCodec, ipld, multihash::Code, Block, DefaultParams, Ipld};
    use network::utils;
    use rpc_server::{
        api::{NetworkPutCarParams, NetworkPutFileParams},
        rpc::api::NetworkGetParams,
    };

    fn create_block(ipld: Ipld) -> Block<DefaultParams> {
        Block::encode(DagCborCodec, Code::Blake3_256, &ipld).unwrap()
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
        let block = create_block(ipld!(&b"hello world"[..]));
        let cid = utils::convert_cid(block.cid().to_bytes());
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
    async fn test_rpc_put_car() {
        setup_logger(LevelFilter::Info);
        let cid = ("bafy2bzaceccu5vqn5xw2morrqa2wtah3w6cs2rnmv64w3ry6st7uelnhxkg6w").to_string();
        println!("{:?}", cid);
        let params = NetworkPutFileParams {
            cid,
            path: "./car_files/ursa_major.car".to_string(),
        };
        match put_file(params).await {
            Ok(v) => {
                println!("Put car file done: {v:?}");
            }
            Err(_e) => {
                println!("There was an error while calling the server. Please Check Server Logs")
            }
        };
    }
}
