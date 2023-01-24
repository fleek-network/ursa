pub const BASE_PATH: &str = "./car_files";

use crate::api::{Car, NetworkInterface, NodeNetworkInterface};
use axum::{
    extract::{DefaultBodyLimit, Multipart, Path},
    http::header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use db::Store;
use futures::io::Cursor;
use fvm_ipld_blockstore::Blockstore;
use hyper::StatusCode;
use libipld::Cid;
use std::{str::FromStr, sync::Arc};
use tokio::task;
use tower_http::limit::RequestBodyLimitLayer;
use tracing::{error, info};

pub fn init<S: Blockstore + Store + Send + Sync + 'static>() -> Router {
    Router::new()
        .route("/ursa/v0/", post(upload_handler::<S>))
        .route("/ursa/v0/:cid", get(get_handler::<S>))
        .route("/ping", get(|| async { "pong" })) // to be used for TLS verification
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(250 * 1024 * 1024)) // 250mb
}

pub enum NetworkError {
    NotFoundError(String),
    InternalError(String),
    BadRequest(String),
}
impl IntoResponse for NetworkError {
    fn into_response(self) -> Response {
        match self {
            NetworkError::NotFoundError(e) => (StatusCode::NOT_FOUND, e).into_response(),
            NetworkError::InternalError(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, e).into_response()
            }
            NetworkError::BadRequest(e) => (StatusCode::BAD_REQUEST, e).into_response(),
        }
    }
}

pub async fn upload_handler<S>(
    Extension(interface): Extension<Arc<NodeNetworkInterface<S>>>,
    mut buf: Multipart,
) -> Result<impl IntoResponse, NetworkError>
where
    S: Blockstore + Store + Send + Sync + 'static,
{
    let upload_task = task::spawn(async move {
        info!("uploading file via http");
        if let Some(field) = buf
            .next_field()
            .await
            .map_err(|e| NetworkError::InternalError(e.to_string()))?
        {
            let content_type = field.content_type().unwrap().to_string();
            if content_type == *"application/vnd.curl.car".to_string() {
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| NetworkError::InternalError(e.to_string()))?;
                let vec_data = data.to_vec();
                let reader = Cursor::new(&vec_data);

                match interface
                    .put_car(Car::new(vec_data.len() as u64, reader))
                    .await
                {
                    Err(err) => {
                        error!("{:?}", err);
                        Err(NetworkError::InternalError(err.to_string()))
                    }
                    Ok(res) => Ok((StatusCode::OK, Json(format!("{res:?}")))),
                }
            } else {
                Err(NetworkError::BadRequest(
                    "Content type do not match. Only .car files can be uploaded".to_string(),
                ))
            }
        } else {
            Err(NetworkError::BadRequest("No files found".to_string()))
        }
    });
    upload_task
        .await
        .map_err(|err| NetworkError::InternalError(err.to_string()))?
}

pub async fn get_handler<S>(
    Path(cid_str): Path<String>,
    Extension(interface): Extension<Arc<NodeNetworkInterface<S>>>,
) -> Result<impl IntoResponse, NetworkError>
where
    S: Blockstore + Store + Send + Sync + 'static,
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
                    format!("attachment; filename=\"{cid_str}.car\"")
                        .parse()
                        .unwrap(),
                );

                Ok(res.status(StatusCode::OK).body(body).unwrap())
            }
            Err(err) => {
                error!("{:?}", err);
                Err(NetworkError::InternalError(err.to_string()))
            }
        };
    } else {
        Err(NetworkError::InternalError(format!(
            "Invalid Cid String, Cannot Parse {cid_str:?} to CID"
        )))
    }
}
