use axum::{body::BoxBody, http::header::CONTENT_TYPE, response::IntoResponse};
use futures::future::BoxFuture;
use hyper::{Body, Request, Response};
use std::{
    convert::Infallible,
    task::{Context, Poll},
};
use tower::Service;
#[derive(Clone)]
pub struct MultiplexService<A, B> {
    http: A,
    rpc: B,
}

impl<A, B> MultiplexService<A, B> {
    pub fn new(http: A, rpc: B) -> Self {
        Self { http, rpc }
    }
}

impl<A, B> Service<Request<Body>> for MultiplexService<A, B>
where
    A: Service<Request<Body>, Error = Infallible>,
    A::Response: IntoResponse,
    A::Future: Send + 'static,
    B: Service<Request<Body>, Error = Infallible>,
    B::Response: IntoResponse,
    B::Future: Send + 'static,
{
    type Response = Response<BoxBody>;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.http.poll_ready(cx) {
            Poll::Ready(Ok(())) => match self.rpc.poll_ready(cx) {
                Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            },
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        if is_rpc_request(&req) {
            let future = self.rpc.call(req);
            Box::pin(async move {
                let res = future.await?;
                Ok(res.into_response())
            })
        } else {
            let future = self.http.call(req);
            Box::pin(async move {
                let res = future.await?;
                Ok(res.into_response())
            })
        }
    }
}

fn is_rpc_request<B>(req: &Request<B>) -> bool {
    req.headers()
        .get(CONTENT_TYPE)
        .map(|content_type| content_type.as_bytes())
        .filter(|content_type| content_type.starts_with(b"application/json"))
        .is_some()
}
