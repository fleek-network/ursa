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

        if let Ok(from_json) = surf::Body::from_json(&rpc_req) {
            debug!("from_json {:?}", from_json);
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

    use crate::functions::get_block;

    use cid::Cid;
    use libipld::{cbor::DagCborCodec, ipld, multihash::Code, Block, DefaultParams, Ipld};
    use network::utils;
    use rpc_server::rpc::api::NetworkGetParams;

    fn create_block(ipld: Ipld) -> Block<DefaultParams> {
        Block::encode(DagCborCodec, Code::Blake3_256, &ipld).unwrap()
    }

    #[tokio::test]
    async fn test_rpc_get_cid() {
        let block = create_block(ipld!(&b"hello world"[..]));
        let cid = utils::convert_cid(block.cid().to_bytes());
        let string_cid = Cid::to_string(&cid);
        println!("{:?}", string_cid);
        let params = NetworkGetParams { cid: string_cid };
        match get_block(params).await {
            Ok(v) => {
                println!("getting cid: {v:?}");
            }
            Err(_e) => println!("error while calling the client"),
        };
    }
}
