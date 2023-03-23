use crate::{cache::Cache, config::ServerConfig, core::Server};
use axum::extract::OriginalUri;
use axum::{
    body::{BoxBody, HttpBody, StreamBody},
    extract::{self, Path},
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
use serde::Deserialize;
use std::path::PathBuf;
use std::str::FromStr;
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
    let mut user_apps = Router::new();
    for location in server_config.clone().location {
        let route_path = format!(
            "/{}/*path",
            location.path.trim_start_matches('/').trim_end_matches('/')
        );
        let user_app = Router::new()
            .route(route_path.as_str(), get(serve_file_handler))
            .layer(Extension(location.root));
        user_apps = user_apps.merge(user_app);
    }
    user_apps
        .route("/*path", get(proxy_pass::<C>))
        .layer(Extension(cache))
        .layer(Extension(client))
        .layer(Extension(Arc::new(server_config)))
}

pub fn init_admin_app<C: Cache>(cache: C, servers: HashMap<String, Server>) -> Router {
    Router::new()
        .route("/purge", post(purge_cache_handler::<C>))
        .route("/reload-tls-config", post(reload_tls_config))
        .layer(Extension(cache))
        .layer(Extension(servers))
}

#[derive(Deserialize, Debug)]
pub struct ReloadTlsConfigPayload {
    name: String,
}

pub async fn reload_tls_config(
    Extension(servers): Extension<HashMap<String, Server>>,
    payload: extract::Json<ReloadTlsConfigPayload>,
) -> Response {
    match servers.get(payload.name.as_str()) {
        None => (
            StatusCode::BAD_REQUEST,
            format!("Unknown server {}", payload.name),
        )
            .into_response(),
        Some(server) => {
            if server.config.tls.is_none() {
                return (
                    StatusCode::BAD_REQUEST,
                    format!("No TLS config found for {}", payload.name),
                )
                    .into_response();
            }
            let server_tls_config = server.config.tls.as_ref().unwrap();
            let cert_path = server_tls_config.cert_path.as_str();
            let key_path = server_tls_config.key_path.as_str();
            if server
                .tls_config
                .as_ref()
                .unwrap()
                .reload_from_pem_file(cert_path, key_path)
                .await
                .is_err()
            {
                return (
                    StatusCode::BAD_REQUEST,
                    format!(
                        "Failed to reload from cert_path: {} and key_path: {}",
                        cert_path, key_path
                    ),
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

async fn serve_file_handler(
    OriginalUri(uri): OriginalUri,
    Extension(root): Extension<String>,
) -> Response {
    let mut file_path = PathBuf::from_str(root.as_str()).unwrap();
    file_path.push(uri.path().trim_start_matches('/'));
    let path_str = file_path.as_os_str().to_str().unwrap();
    println!("{path_str}");
    match tokio::fs::read(path_str).await {
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        Ok(file) => Response::builder()
            .status(StatusCode::OK)
            .body(axum::body::boxed(axum::body::Full::from(file)))
            .unwrap(),
    }
}
