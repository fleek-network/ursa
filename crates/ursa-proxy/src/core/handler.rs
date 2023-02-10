use crate::{cache::Cache, config::ServerConfig, core::Server};
use axum::{
    body::{BoxBody, HttpBody, StreamBody},
    extract::{self, Path},
    headers::CacheControl,
    http::{response::Parts, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Extension, Router, TypedHeader,
};
use bytes::BufMut;
use hyper::{
    client::{self, HttpConnector},
    Body,
};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    io::{duplex, AsyncWriteExt},
    task,
};
use tokio_util::io::ReaderStream;
use tracing::{error, info, warn};

type Client = client::Client<HttpConnector, Body>;

pub async fn init_server_app<C: Cache>(
    server_config: ServerConfig,
    cache: C,
    client: Client,
) -> Router {
    Router::new()
        .route("/*path", get(proxy_pass::<C>))
        .layer(Extension(cache.clone()))
        .layer(Extension(client.clone()))
        .layer(Extension(Arc::new(server_config.clone())))
}

#[derive(Deserialize, Debug)]
pub struct ReloadTlsPayload {
    name: String,
}

pub async fn reload_tls(
    Extension(servers): Extension<HashMap<String, Server>>,
    payload: extract::Json<ReloadTlsPayload>,
) -> Response {
    match servers.get(payload.name.as_str()) {
        None => (
            StatusCode::BAD_REQUEST,
            format!("Unknown server {}", payload.name),
        )
            .into_response(),
        Some(server) => {
            if server.config.reload_cert_path.is_none() || server.config.reload_key_path.is_none() {
                return StatusCode::OK.into_response();
            }
            if server
                .tls_config
                .as_ref()
                .unwrap()
                .reload_from_pem_file(
                    server.config.reload_cert_path.as_ref().unwrap(),
                    server.config.reload_key_path.as_ref().unwrap(),
                )
                .await
                .is_err()
            {
                return (
                    StatusCode::BAD_REQUEST,
                    "Failed to reload from path".to_string(),
                )
                    .into_response();
            }
            StatusCode::OK.into_response()
        }
    }
}

pub async fn purge_cache_handler<C: Cache>(Extension(cache): Extension<C>) -> StatusCode {
    cache.purge();
    StatusCode::OK
}

pub async fn proxy_pass<C: Cache>(
    Path(path): Path<String>,
    cache_control: Option<TypedHeader<CacheControl>>,
    Extension(config): Extension<Arc<ServerConfig>>,
    Extension(client): Extension<Client>,
    Extension(cache_client): Extension<C>,
) -> Response {
    let no_cache = cache_control.map_or(false, |c| c.no_cache());
    if !no_cache {
        if let Some(resp) = cache_client.get(path.clone()) {
            info!("Cache hit");
            return resp;
        }
        info!("Cache miss for {path}");
    }

    let endpoint = format!("http://{}/{}", config.proxy_pass, path);
    let uri = match endpoint.parse::<Uri>() {
        Ok(uri) => uri,
        Err(e) => return e.to_string().into_response(),
    };
    info!("Sending request to {endpoint}");

    let reader = match client.get(uri).await {
        Ok(resp) => match resp.into_parts() {
            (
                Parts {
                    status: StatusCode::OK,
                    ..
                },
                mut body,
            ) => {
                let (mut writer, reader) = duplex(100);
                task::spawn(async move {
                    let mut bytes = Vec::new();
                    while let Some(buf) = body.data().await {
                        match buf {
                            Ok(buf) => {
                                if let Err(e) = writer.write_all(buf.as_ref()).await {
                                    warn!("Failed to write to stream for {e:?}");
                                }
                                bytes.put(buf);
                            }
                            Err(e) => {
                                error!("Failed to read stream for {e:?}");
                                return;
                            }
                        }
                    }
                    cache_client.insert(path, bytes);
                });
                reader
            }
            (parts, body) => {
                return Response::from_parts(parts, BoxBody::new(StreamBody::new(body)))
            }
        },
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    StreamBody::new(ReaderStream::new(reader)).into_response()
}
