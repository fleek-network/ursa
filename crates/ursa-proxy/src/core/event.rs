use axum::{body::Body, http::Response};

pub enum ProxyEvent {
    UpstreamData(Vec<u8>),
    Error(String),
}
