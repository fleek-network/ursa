use axum::{body::Body, http::Response};

pub enum ProxyEvent {
    Upstream(Response<Body>),
    Error(String),
}
