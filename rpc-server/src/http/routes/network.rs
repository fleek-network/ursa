pub const BASE_PATH: &str = "./car_files";
pub const MAX_FILE_SIZE: u64 = 1024 * 1024 * 100;

use std::io::Cursor;
use crate::api::{NetworkInterface, NodeNetworkInterface};
use axum::{
    extract::{ContentLengthLimit, Multipart},
    response::IntoResponse,
    routing::post,
    Extension, Json, Router,
};

use hyper::StatusCode;
use ipld_blockstore::BlockStore;
use std::sync::Arc;
use tracing::{error, info};

pub fn init<S: BlockStore + Sync + Send + 'static>() -> Router {
    Router::new().route("/upload", post(upload_handler::<S>))
}

pub async fn upload_handler<S>(
    ContentLengthLimit(mut buf): ContentLengthLimit<Multipart, { MAX_FILE_SIZE }>,
    Extension(interface): Extension<Arc<NodeNetworkInterface<S>>>,
) -> impl IntoResponse
where
    S: BlockStore + Sync + Send + 'static,
{
    info!("uploading file via http");
    if let Some(field) = buf.next_field().await.unwrap() {
        let content_type = field.content_type().unwrap().to_string();
        if content_type == "application/vnd.curl.car".to_string() {
            let data = field.bytes().await.unwrap();
            let vec_data = data.to_vec();
            let reader = Cursor::new(&vec_data);

            return match interface.put_car(reader).await {
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
