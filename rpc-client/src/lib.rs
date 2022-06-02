use std::net::TcpStream;

use anyhow::Error;
use async_trait::async_trait;
use rpc::UrsaRpc;
use serde::Serialize;

mod rpc;

#[derive(Clone, Debug, Serialize)]
pub struct Params<T: Serialize>(pub T);

#[derive(Clone, Debug)]
pub struct URL {
    pub port: String,
    pub domain: String,
}

impl URL {
    pub fn new(domain: &str, port: &str) -> URL {
        URL {
            port: port.to_owned(),
            domain: domain.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RpcClient<T: Serialize> {
    pub method: String,
    pub params: Params<T>,
}

impl<T: Serialize> RpcClient<T> {
    pub fn new(method: &str, params: Params<T>) -> RpcClient<T> {
        RpcClient {
            method: method.to_owned(),
            params,
        }
    }

    pub fn send(&self, url: &URL) -> serde_json::Value {
        todo!()
    }
}

#[async_trait]
impl<T: Serialize + Clone + Send + Sync + 'static> UrsaRpc<T> for RpcClient<T> {
    async fn put(&self) -> Result<(), Error> {
        todo!()
    }

    async fn get(&self) -> Result<(), Error> {
        todo!()
    }
}
