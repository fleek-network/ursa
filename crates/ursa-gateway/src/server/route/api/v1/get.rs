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
use tracing::{info_span, Instrument};

use crate::{
    config::GatewayConfig, server::model::HttpResponse, util::error::Error,
    worker::cache::server::ServerCache,
};

pub async fn get_car_handler<Cache: ServerCache>(
    Path(cid): Path<String>,
    cache_control: Option<TypedHeader<CacheControl>>,
    Extension(cache): Extension<Arc<RwLock<Cache>>>,
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
    let no_cache = cache_control.map_or(false, |c| c.no_cache());
    match cache
        .read()
        .await
        .get_announce(&cid, no_cache)
        .instrument(span)
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
                (
                    header::CACHE_CONTROL,
                    &(if no_cache {
                        "no-cache".into()
                    } else {
                        format!(
                            "public, max-age={}, immutable",
                            config.read().await.server.cache_control_max_age
                        )
                    }),
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
