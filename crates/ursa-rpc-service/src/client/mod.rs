mod rpc;

use crate::api::{
    NetworkGetFileParams, NetworkGetParams, NetworkGetResult, NetworkPutFileParams,
    NetworkPutFileResult, NETWORK_GET, NETWORK_GET_FILE, NETWORK_PUT_FILE,
};
use crate::config::DEFAULT_PORT;
use jsonrpc_v2::{Error, RequestObject};
pub use rpc::*;
use serde::de::DeserializeOwned;
use std::net::Ipv4Addr;
use tracing::{debug, error, info};

pub type Result<T> = anyhow::Result<T, Error>;

pub struct Client {
    server_addr: Ipv4Addr,
    server_port: u16,
}

impl Default for Client {
    fn default() -> Self {
        Client::new(Ipv4Addr::from([0, 0, 0, 0]), DEFAULT_PORT)
    }
}

impl Client {
    pub fn new(server_addr: Ipv4Addr, server_port: u16) -> Self {
        Self {
            server_addr,
            server_port,
        }
    }

    pub fn set_port(&mut self, port: u16) {
        self.server_port = port;
    }

    pub async fn get_block(&self, params: NetworkGetParams) -> Result<NetworkGetResult> {
        let rpc_req = create_request(NETWORK_GET, params)?;
        self.call(rpc_req, RpcMethod::Post).await
    }

    pub async fn get_file(&self, params: NetworkGetFileParams) -> Result<()> {
        let rpc_req = create_request(NETWORK_GET_FILE, params)?;
        self.call(rpc_req, RpcMethod::Put).await
    }

    pub async fn put_file(&self, params: NetworkPutFileParams) -> Result<NetworkPutFileResult> {
        let rpc_req = create_request(NETWORK_PUT_FILE, params)?;
        self.call(rpc_req, RpcMethod::Put).await
    }

    /// Utility method for sending RPC requests over HTTP
    async fn call<R>(&self, rpc_req: RequestObject, method: RpcMethod) -> Result<R>
    where
        R: DeserializeOwned,
    {
        let port = self.server_addr.to_string();
        let addr = self.server_port;
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
            error!("[RPCClient] - There was an error while serializing the rpc request");
            Err(Error::Full {
                data: None,
                code: 200,
                message: "There was an error while serializing the rpc request".to_string(),
            })
        }
    }
}
