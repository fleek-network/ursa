use std::{str::FromStr, sync::Arc};

use axum::body::StreamBody;
use axum::http::response::Parts;
use axum::{
    extract::Path,
    headers::CacheControl,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json, TypedHeader,
};
use hyper::Body;
use libipld::Cid;
use serde_json::{json, Value};
use tokio::sync::RwLock;
use tracing::{error, info_span, Instrument};

use crate::resolver::Resolver;
use crate::{
    config::GatewayConfig, server::model::HttpResponse, util::error::Error,
    worker::cache::server::ServerCache,
};

pub async fn get_car_handler(
    Path(cid): Path<String>,
    Extension(resolver): Extension<Arc<Resolver>>,
    Extension(config): Extension<Arc<RwLock<GatewayConfig>>>,
) -> Response {
    let span = info_span!("Get car handler");
    if Cid::from_str(&cid).is_err() {
        return error_handler(
            StatusCode::BAD_REQUEST,
            format!("Invalid cid string, cannot parse {cid} to CID"),
        )
        .into_response();
    };

    let body = match resolver.resolve_content(&cid).instrument(span).await {
        Ok(resp) => match resp.into_parts() {
            (
                Parts {
                    status: StatusCode::OK,
                    ..
                },
                body,
            ) => body,
            (parts, body) => {
                error!("Error requested provider with parts: {parts:?} and body: {body:?}");
                return error_handler(parts.status, "Error requested provider".to_string())
                    .into_response();
            }
        },
        Err(_) => {
            return error_handler(
                StatusCode::INTERNAL_SERVER_ERROR,
                "There was an error".to_string(),
            )
            .into_response()
        }
    };

    (
        [
            (
                header::CONTENT_TYPE,
                "application/vnd.curl.car; charset=utf-8",
            ),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{cid}.car\""),
            ),
        ],
        StreamBody::new(body),
    )
        .into_response()
}

fn error_handler(status_code: StatusCode, message: String) -> (StatusCode, Json<Value>) {
    (
        status_code,
        Json(json!(HttpResponse {
            message: Some(message),
        })),
    )
}
