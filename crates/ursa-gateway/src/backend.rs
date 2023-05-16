use crate::util::Client;
use anyhow::{Error, Result};
use axum::response::{IntoResponse, Response};
use hyper::{Body, Request, Uri};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::Service;

// Service that will query the edge nodes for the content.
#[derive(Clone)]
pub struct Backend {
    uri: Uri,
    client: Client,
}

impl Backend {
    pub fn new(uri: Uri, client: Client) -> Self {
        Self { uri, client }
    }
}

impl Service<Request<Body>> for Backend {
    type Response = Response;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response>>>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: Request<Body>) -> Self::Future {
        let this = self.clone();
        Box::pin(async move {
            match this.client.get(this.uri).await {
                Ok(response) => Ok(response.into_response()),
                Err(e) => Err(e.into()),
            }
        })
    }
}
