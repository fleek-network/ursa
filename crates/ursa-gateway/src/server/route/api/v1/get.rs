use std::{str::FromStr, sync::Arc};

use axum::{
    body::StreamBody,
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Extension,
    Json,
};
use cid::Cid;
use serde_json::{json, Value};
use tokio::{
    io::{duplex, AsyncWriteExt},
    sync::RwLock,
    task::spawn,
};
use tokio_util::io::ReaderStream;
use tracing::error;

use crate::{server::model::HttpResponse, worker::cache::ServerCache};

pub async fn get_block_handler<Cache: ServerCache>(
    Path(cid): Path<String>,
    Extension(cache): Extension<Arc<RwLock<Cache>>>,
) -> Response {
    if Cid::from_str(&cid).is_err() {
        return error_handler(
            StatusCode::BAD_REQUEST,
            format!("Invalid cid string, cannot parse {cid} to CID"),
        )
        .into_response();
    };

    match cache.read().await.get_announce(&cid).await {
        Ok(data) => {
            let (mut w, r) = duplex(2 * 1024 * 1024); // 2MiB
            spawn(async move {
                if let Err(e) = w.write_all(data.as_slice()).await {
                    error!("Failed to write stream for {e:?}");
                }
            });
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
                StreamBody::new(ReaderStream::new(r)),
            )
                .into_response()
        }
        Err(message) =>
            error_handler(StatusCode::INTERNAL_SERVER_ERROR, message.to_string()).into_response(),
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
