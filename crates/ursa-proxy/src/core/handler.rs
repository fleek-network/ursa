use crate::config::ServerConfig;
use axum::{
    body::StreamBody,
    extract::Path,
    http::{response::Parts, StatusCode, Uri},
    response::{IntoResponse, Response},
    Extension,
};
use hyper::Client;
use std::sync::Arc;
use tracing::info;

pub async fn proxy_pass(
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
