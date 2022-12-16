pub const BASE_PATH: &str = "./car_files";

use crate::api::{NetworkInterface, NodeNetworkInterface};
use anyhow::{anyhow, Error};
use axum::{
    extract::{Multipart, Path},
    http::header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use cid::Cid;
use db::Store as Store_;
use futures::io::Cursor;
use fvm_ipld_blockstore::Blockstore;
use hyper::StatusCode;
use std::{str::FromStr, sync::Arc};
use tracing::{error, info};

pub fn init<S: Blockstore + Store_ + Send + Sync + 'static>() -> Router {
    Router::new()
        .route("/", post(upload_handler::<S>))
        .route("/:cid", get(get_handler::<S>))
}

pub enum NetworkError {
    NotFoundError(Error),
    InternalError(Error),
}
impl IntoResponse for NetworkError {
    fn into_response(self) -> Response {
        match self {
            NetworkError::NotFoundError(e) => {
                (StatusCode::NOT_FOUND, e.to_string()).into_response()
            }
            NetworkError::InternalError(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
            }
        }
    }
}

pub async fn upload_handler<S>(
    mut buf: Multipart,
    Extension(interface): Extension<Arc<NodeNetworkInterface<S>>>,
) -> impl IntoResponse
where
    S: Blockstore + Store_ + Send + Sync + 'static,
{
    info!("uploading file via http");
    if let Some(field) = buf.next_field().await.unwrap() {
        let content_type = field.content_type().unwrap().to_string();
        if content_type == *"application/vnd.curl.car".to_string() {
            let data = field.bytes().await.unwrap();
            let vec_data = data.to_vec();
            let reader = Cursor::new(&vec_data);

            match interface.put_car(reader).await {
                Err(err) => {
                    error!("{:?}", err);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(format!("{:?}", err)),
                    )
                }
                Ok(res) => (StatusCode::OK, Json(format!("{:?}", res))),
            }
        } else {
            (
                StatusCode::BAD_REQUEST,
                Json("Content type do not match. Only .car files can be uploaded".to_string()),
            )
        }
    } else {
        (StatusCode::BAD_REQUEST, Json("No files found".to_string()))
    }
}

pub async fn get_handler<S>(
    Path(cid_str): Path<String>,
    Extension(interface): Extension<Arc<NodeNetworkInterface<S>>>,
) -> Result<impl IntoResponse, NetworkError>
where
    S: Blockstore + Store_ + Send + Sync + 'static,
{
    info!("Streaming file over http");
    if let Ok(cid) = Cid::from_str(&cid_str) {
        let mut res = Response::builder();
        return match interface.stream(cid).await {
            Ok(body) => {
                let headers = res.headers_mut().unwrap();
                headers.insert(
                    CONTENT_TYPE,
                    "application/vnd.curl.car; charset=utf-8".parse().unwrap(),
                );
                headers.insert(
                    CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}.car\"", cid_str)
                        .parse()
                        .unwrap(),
                );

                Ok(res.status(StatusCode::OK).body(body).unwrap())
            }
            Err(err) => {
                error!("{:?}", err);
                Err(NetworkError::InternalError(anyhow!("{}", err)))
            }
        };
    } else {
        Err(NetworkError::InternalError(anyhow!(
            "Invalid Cid String, Cannot Parse {} to CID",
            &cid_str
        )))
    }
}
