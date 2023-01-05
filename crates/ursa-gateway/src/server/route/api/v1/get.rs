use std::{str::FromStr, sync::Arc};

use axum::{
    extract::Path,
    headers::CacheControl,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json, TypedHeader,
};
use cid::Cid;
use serde_json::{json, Value};
use tokio::sync::RwLock;

use crate::{server::model::HttpResponse, util::error::Error, worker::cache::server::ServerCache};

pub async fn get_car_handler<Cache: ServerCache>(
    Path(cid): Path<String>,
    cache_control: Option<TypedHeader<CacheControl>>,
    Extension(cache): Extension<Arc<RwLock<Cache>>>,
) -> Response {
    if Cid::from_str(&cid).is_err() {
        return error_handler(
            StatusCode::BAD_REQUEST,
            format!("Invalid cid string, cannot parse {cid} to CID"),
        )
        .into_response();
    };

    match cache
        .read()
        .await
        .get_announce(&cid, cache_control.map_or(false, |c| c.no_cache()))
        .await
    {
        Ok(stream) => (
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
            stream,
        )
            .into_response(),
        Err(Error::Upstream(status, message)) => error_handler(status, message).into_response(),
        Err(Error::Internal(message)) => {
            error_handler(StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
        }
    }
}

fn error_handler(status_code: StatusCode, message: String) -> (StatusCode, Json<Value>) {
    (
        status_code,
        Json(json!(HttpResponse {
            message: Some(message),
        })),
    )
}
