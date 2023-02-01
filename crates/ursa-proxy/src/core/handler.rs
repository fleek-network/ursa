use crate::{cache::Cache, config::ServerConfig, core::event::ProxyEvent};
use axum::{
    body::{BoxBody, HttpBody, StreamBody},
    extract::Path,
    http::{response::Parts, StatusCode, Uri},
    response::{IntoResponse, Response},
    Extension,
};
use bytes::BufMut;
use hyper::Client;
use std::sync::Arc;
use tokio::{
    io::{duplex, AsyncWriteExt},
    spawn,
    sync::oneshot,
};
use tokio_util::io::ReaderStream;
use tracing::{error, info, warn};

pub async fn proxy_pass<C: Cache>(
    Path(path): Path<String>,
    Extension(config): Extension<Arc<ServerConfig>>,
    Extension(cache_client): Extension<C>,
) -> Response {
    let (tx, rx) = oneshot::channel();
    cache_client
        .handle_proxy_event(ProxyEvent::GetRequest {
            key: path.clone(),
            sender: tx,
        })
        .await;
    match rx.await {
        Ok(Some(resp)) => {
            info!("Cache hit");
            return resp;
        }
        Err(e) => {
            error!("Failed to receive {e:?}");
        }
        _ => {}
    }
    info!("Cache miss for {path}");
    let endpoint = format!("http://{}/{}", config.proxy_pass, path);
    let uri = match endpoint.parse::<Uri>() {
        Ok(uri) => uri,
        Err(e) => return e.to_string().into_response(),
    };
    info!("Sending request to {endpoint}");
    let client = Client::new();
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
                spawn(async move {
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
                    cache_client
                        .handle_proxy_event(ProxyEvent::UpstreamData {
                            key: path,
                            value: bytes,
                        })
                        .await
                });
                reader
            }
            (parts, body) => {
                return Response::from_parts(parts, BoxBody::new(StreamBody::new(body)))
            }
        },
        Err(e) => {
            cache_client
                .handle_proxy_event(ProxyEvent::Error(e.to_string()))
                .await;
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };
    StreamBody::new(ReaderStream::new(reader)).into_response()
}

pub async fn proxy_pass_no_cache(
    Path(path): Path<String>,
    Extension(config): Extension<Arc<ServerConfig>>,
) -> Response {
    let endpoint = format!("http://{}/{}", config.proxy_pass, path);
    info!("Sending request to {endpoint}");
    let uri = match endpoint.parse::<Uri>() {
        Ok(uri) => uri,
        Err(e) => return e.to_string().into_response(),
    };
    let client = Client::new();
    match client.get(uri).await {
        Ok(resp) => match resp.into_parts() {
            (
                Parts {
                    status: StatusCode::OK,
                    ..
                },
                body,
            ) => StreamBody::new(body).into_response(),
            (status, body) => (status, StreamBody::from(body)).into_response(),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
