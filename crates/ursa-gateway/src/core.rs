use crate::{
    indexer::IndexerCommand,
    resolver::{Config, Resolver},
};
use anyhow::{anyhow, Error, Result};
use axum::response::Response;
use hyper::{Body, Request};
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::ready,
    task::{Context, Poll},
};
use tokio::sync::mpsc::Sender;
use tower::{balance::p2c::MakeBalance, Service};

// TODO: Add a state that is cheap to copy.
struct Gateway<S, Req> {
    config: Arc<Config>,
    tx: Sender<IndexerCommand<S, Req>>,
}

impl<S> Service<Request<Body>> for Gateway<S, Request<Body>>
where
    S: Service<Request<Body>, Response = Response, Error = Error> + Clone + Unpin + 'static,
    <S as Service<Request<Body>>>::Future: Unpin,
{
    type Response = Response;
    type Error = Error;
    type Future = ResponseFuture<S>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        ResponseFuture {
            request: Some(req),
            balance: MakeBalance::new(Resolver::new(self.tx.clone(), self.config.clone())),
        }
    }
}

struct ResponseFuture<S> {
    request: Option<Request<Body>>,
    balance: MakeBalance<Resolver<S, Request<Body>>, Request<Body>>,
}

impl<S> Future for ResponseFuture<S>
where
    S: Service<Request<Body>, Response = Response, Error = Error> + Clone + Unpin + 'static,
    <S as Service<Request<Body>>>::Future: Unpin,
{
    type Output = Result<S::Response>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let cid = self
            .request
            .as_ref()
            .map(|r| r.uri().path().trim_start_matches('/'))
            .expect("There to be a request")
            .to_string();
        let mut balance = match ready!(Pin::new(&mut self.balance.call(cid)).poll(cx)) {
            Ok(balance) => balance,
            Err(e) => return Poll::Ready(Err(e)),
        };
        if let Err(e) = ready!(balance.poll_ready(cx)) {
            // TODO: Add logging.
            return Poll::Ready(Err(anyhow!("{e:?}")));
        }

        let request = self.request.take().expect("There to be a request");
        let response = match ready!(Pin::new(&mut balance.call(request)).poll(cx)) {
            Ok(r) => r,
            // TODO: Can we convert from BoxError?
            Err(e) => return Poll::Ready(Err(anyhow!("{e:?}"))),
        };
        Poll::Ready(Ok(response))
    }
}
