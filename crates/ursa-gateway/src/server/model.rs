use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct HttpResponse {
    pub message: Option<String>,
    pub data: Option<Vec<u8>>,
}
