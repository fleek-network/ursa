use std::{str::FromStr, sync::Arc};

use axum::body::StreamBody;
use axum::http::response::Parts;
use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use libipld::Cid;
use serde_json::{json, Value};
use tracing::{error, info_span, Instrument};

use crate::resolver::Resolver;
use crate::server::model::HttpResponse;
use crate::util::error::Error;

pub async fn get_car_handler(
    Path(cid): Path<String>,
    Extension(resolver): Extension<Arc<Resolver>>,
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
        Err(Error::Internal(message)) => {
            return error_handler(StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
        }
        Err(Error::Upstream(status, message)) => return (status, message).into_response(),
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
