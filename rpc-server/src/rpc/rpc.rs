use std::sync::Arc;

use anyhow::Ok;
use jsonrpc_v2::{Data, MapRouter, Server};
use serde::Serialize;

use crate::config::RpcConfig;

use super::{api::NetworkInterface, method::get_handler::get_handler};

#[derive(Clone)]
pub struct RpcServer(Arc<Server<MapRouter>>);

impl RpcServer {
    pub fn new<I, T>(config: &RpcConfig, interface: Arc<I>) -> Self
    where
        I: NetworkInterface<T>,
        T: Serialize + 'static,
    {
        let server = Server::new()
            .with_data(Data::new(interface))
            .with_method("ursa_get", get_handler::<I, T>);

        RpcServer(server.finish())
    }

    pub fn handler() -> anyhow::Result<()> {
        Ok(())
    }
}
