use crate::{cache::Cache, config::ServerConfig, core::Server};
use axum::{
    body::{BoxBody, HttpBody, StreamBody},
    extract::Path,
    headers::CacheControl,
    http::{response::Parts, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Router, TypedHeader,
};
use bytes::BufMut;
use hyper::{
    client::{self, HttpConnector},
    Body,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::{
    io::{duplex, AsyncWriteExt},
    task,
};
use tokio_util::io::ReaderStream;
use tower_http::services::ServeDir;
use tracing::{error, info, warn};

type Client = client::Client<HttpConnector, Body>;

pub fn init_server_app<C: Cache>(
    server_config: Arc<ServerConfig>,
    cache: C,
    client: Client,
) -> Router {
    let mut user_app = Router::new();
    if let Some(path) = &server_config.serve_dir_path {
        let directory = if path.is_absolute() {
            path.strip_prefix("/")
                .expect("To start with slash")
                .to_path_buf()
        } else {
            path.clone()
        };
        let directory_str = directory
            .to_str()
            .expect("Path to be a valid unicode string");
        // We have to check for this because we already have a route for "/".
        if directory_str.is_empty() {
            panic!("Invalid directory to serve")
        }
        let route_path = format!("/{}", directory_str);
        user_app = user_app.nest_service(route_path.as_str(), ServeDir::new(directory));
    }
    user_app
        .route("/*path", get(proxy_pass::<C>))
        .layer(Extension(cache))
        .layer(Extension(client))
        .layer(Extension(server_config))
}

pub fn init_admin_app<C: Cache>(cache: C, servers: Vec<Server>) -> Router {
    Router::new()
        .route("/purge", post(purge_cache_handler::<C>))
        .route("/reload-tls-config", post(reload_tls_config))
        .layer(Extension(cache))
        .layer(Extension(servers))
}

pub async fn reload_tls_config(Extension(servers): Extension<Vec<Server>>) -> StatusCode {
    for server in servers {
        if server.config.tls.is_none() {
            continue;
        }
        let server_tls_config = server.config.tls.as_ref().unwrap();
        match (
            server_tls_config.cert_path.to_str(),
            server_tls_config.key_path.to_str(),
        ) {
            (Some(cert_path), Some(key_path)) => {
                if let Err(e) = server
                    .tls_config
                    .as_ref()
                    .unwrap()
                    .reload_from_pem_file(cert_path, key_path)
                    .await
                {
                    error!("Failed to reload from pem file {}", e);
                }
            }
            _ => {
                error!("Invalid paths");
                continue;
            }
        }
    }
    StatusCode::OK
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
