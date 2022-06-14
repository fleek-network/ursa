use std::sync::Arc;

use axum::{Extension, Json};
use jsonrpc_v2::{Data, MapRouter, RequestObject, ResponseObjects, Server};
use serde::Serialize;

use crate::config::RpcConfig;

use super::{api::NetworkInterface, routes::network};

#[derive(Clone)]
pub struct RpcServer(Arc<Server<MapRouter>>);

pub async fn http_handler(
    Json(req): Json<RequestObject>,
    Extension(server): Extension<RpcServer>,
) -> Json<ResponseObjects> {
    let res = server.0.handle(req).await;
    Json(res)
}

impl RpcServer {
    pub fn new<I, T>(config: &RpcConfig, interface: Arc<I>) -> Self
    where
        I: NetworkInterface<T>,
        T: Serialize + 'static,
    {
        let server = Server::new()
            .with_data(Data::new(interface))
            .with_method("ursa_get_cid", network::get_cid_handler::<I, T>)
            .with_method("ursa_put_car", network::put_car_handler::<I, T>);

        RpcServer(server.finish())
    }
}
