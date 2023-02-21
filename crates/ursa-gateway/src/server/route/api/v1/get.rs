use std::{str::FromStr, sync::Arc};

use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, Json,
};
use libipld::Cid;
use serde_json::{json, Value};
use tracing::{info_span, Instrument};

use crate::{resolver::Picker, server::model::HttpResponse, util::error::Error};

pub async fn check_car_handler(
    Path(cid): Path<String>,
    Extension(resolver): Extension<Arc<Picker>>,
) -> StatusCode {
    let span = info_span!("Check car handler");
    if Cid::from_str(&cid).is_err() {
        return StatusCode::BAD_REQUEST;
    };
    match resolver.resolve_content(&cid).instrument(span).await {
        Ok(resp) => resp.status(),
        Err(Error::Internal(_)) => StatusCode::INTERNAL_SERVER_ERROR,
        Err(Error::Upstream(status, _)) => status,
    }
}

pub async fn get_car_handler(
    Path(cid): Path<String>,
    Extension(resolver): Extension<Arc<Picker>>,
) -> Response {
    let span = info_span!("Get car handler");
    if Cid::from_str(&cid).is_err() {
        return error_handler(
            StatusCode::BAD_REQUEST,
            format!("Invalid cid string, cannot parse {cid} to CID"),
        )
        .into_response();
    };

    match resolver.resolve_content(&cid).instrument(span).await {
        Ok(resp) => resp.into_response(),
        Err(Error::Internal(message)) => {
            error_handler(StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
        }
        Err(Error::Upstream(status, message)) => error_handler(status, message).into_response(),
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
