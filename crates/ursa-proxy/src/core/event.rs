use axum::{body::Body, http::Response};

pub enum ProxyEvent {
    UpstreamData(String, Vec<u8>),
    Timer,
    Error(String),
}
