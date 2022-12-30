use axum::http::StatusCode;

pub mod timer;

#[derive(Debug)]
pub enum Error {
    Upstream(StatusCode, String),
    Internal(String),
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Error::Internal(e.to_string())
    }
}
