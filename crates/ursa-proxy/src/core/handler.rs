use crate::{
    cache::Cache,
    config::{ServerConfig, DEFAULT_UPSTREAM_BUF_SIZE},
    core::Server,
};
use axum::{
    body::{BoxBody, HttpBody, StreamBody},
    extract::Path,
    headers::CacheControl,
    http::{response::Parts, HeaderName, HeaderValue, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::{get, post, IntoMakeService},
    Extension, Router, ServiceExt, TypedHeader,
};
use axum_tracing_opentelemetry::opentelemetry_tracing_layer;
use bytes::BufMut;
use hyper::{
    client::{self, HttpConnector},
    Body,
};
use std::{str::FromStr, sync::Arc};
use tokio::{
    io::{duplex, AsyncWriteExt},
    task,
};
use tokio_util::io::ReaderStream;
use tower_http::{
    normalize_path::NormalizePath,
    services::ServeDir,
    set_header::SetResponseHeaderLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::{debug, error, info, info_span, warn, Instrument, Level};

type Client = client::Client<HttpConnector, Body>;

pub fn init_server_app<C: Cache>(
    server_config: Arc<ServerConfig>,
    cache: C,
    client: Client,
) -> IntoMakeService<NormalizePath<Router>> {
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

    user_app = user_app
        .route("/*path", get(proxy_pass::<C>))
        .layer(Extension(cache))
        .layer(Extension(client))
        .layer(Extension(server_config.clone()))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().include_headers(true))
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .include_headers(true)
                        .latency_unit(tower_http::LatencyUnit::Micros),
                ),
        )
        .layer(opentelemetry_tracing_layer());

    if let Some(headers) = &server_config.add_header {
        for (header, values) in headers.iter() {
            for value in values {
                let value = value.clone();
                user_app = user_app.layer(SetResponseHeaderLayer::appending(
                    HeaderName::from_str(header).expect("Header name to be valid"),
                    move |_: &Response| {
                        Some(HeaderValue::from_str(&value).expect("Header value to be valid"))
                    },
                ));
            }
        }
    }

    NormalizePath::trim_trailing_slash(user_app).into_make_service()
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

    match client
        .get(uri)
        .instrument(info_span!("request_upstream"))
        .await
    {
        Ok(resp) => match resp.into_parts() {
            (
                parts @ Parts {
                    status: StatusCode::OK,
                    ..
                },
                mut body,
            ) => {
                let max_size_cache_entry = config.max_size_cache_entry.unwrap_or(0);
                let (mut writer, reader) = duplex(
                    config
                        .upstream_buf_size
                        .unwrap_or(DEFAULT_UPSTREAM_BUF_SIZE),
                );
                let stream_body_fut = async move {
                    let mut bytes = Vec::new();
                    let mut skip_cache = false;
                    while let Some(buf) = body.data().await {
                        match buf {
                            Ok(buf) => {
                                if let Err(e) = writer.write_all(buf.as_ref()).await {
                                    warn!("Failed to write to stream for {e:?}");
                                }
                                if !skip_cache {
                                    bytes.put(buf);
                                    skip_cache = max_size_cache_entry > 0
                                        && bytes.len() > max_size_cache_entry;
                                }
                            }
                            Err(e) => {
                                error!("Failed to read stream for {e:?}");
                                return;
                            }
                        }
                    }
                    if skip_cache {
                        debug!("Data exceeds max size for a cache entry");
                        return;
                    }
                    cache_client.insert(path, bytes);
                };
                task::spawn(stream_body_fut.instrument(info_span!("stream_body_from_upstream")));
                Response::from_parts(
                    parts,
                    BoxBody::new(StreamBody::new(ReaderStream::new(reader))),
                )
            }
            (parts, body) => Response::from_parts(parts, BoxBody::new(StreamBody::new(body))),
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
