use crate::cache::CacheClient;
use crate::config::ServerConfig;
use crate::core::event::ProxyEvent;
use axum::body::{BoxBody, HttpBody};
use axum::{
    body::StreamBody,
    extract::Path,
    http::{response::Parts, StatusCode, Uri},
    response::{IntoResponse, Response},
    Extension,
};
use bytes::{BufMut, Bytes};
use hyper::{Client, Error};
use std::sync::Arc;
use tokio::io::{duplex, AsyncWriteExt};
use tokio::spawn;
use tokio_util::io::ReaderStream;
use tracing::{error, info, warn};

pub async fn proxy_pass<C: CacheClient>(
    Path(path): Path<String>,
    Extension(config): Extension<Arc<ServerConfig>>,
    Extension(cache_client): Extension<C>,
) -> Response {
    if let Ok(Some(resp)) = cache_client.query_cache(&path, false).await {
        info!("Cache hit");
        return resp;
    }
    info!("Cache miss for {path}");
    let endpoint = format!("http://{}/{}", config.proxy_pass, path);
    info!("Sending request to {endpoint}");
    let uri = match endpoint.parse::<Uri>() {
        Ok(uri) => uri,
        Err(e) => return e.to_string().into_response(),
    };
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
                        .handle_proxy_event(ProxyEvent::UpstreamData(bytes))
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
