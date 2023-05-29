use crate::{
    backend::Backend,
    resolve::{CIDResolver, Cid, Cluster, Resolver},
    types::Worker,
};
use anyhow::{Error, Result};
use axum::response::Response;
use hyper::{Body, Request};
use moka::sync::Cache;
use std::{
    future::Future,
    pin::Pin,
    task::ready,
    task::{Context, Poll},
};
use tower::{
    balance::p2c::{Balance, MakeFuture},
    load::PeakEwmaDiscover,
    Service,
};

type ResolutionFuture = MakeFuture<
    Pin<Box<dyn Future<Output = Result<PeakEwmaDiscover<Cluster<Backend, Request<Body>>>>>>>,
    Request<Body>,
>;
type GatewayService = Worker<
    Balance<PeakEwmaDiscover<Cluster<Backend, Request<Body>>>, Request<Body>>,
    Request<Body>,
>;

/// Service that will run in Hyper/Axum.
#[derive(Clone)]
struct Server {
    cache: Cache<Cid, GatewayService>,
    resolver: Resolver<CIDResolver>,
}

impl Server {
    pub fn _new(resolver: CIDResolver, cache: Cache<Cid, GatewayService>) -> Self {
        Self {
            cache,
            resolver: Resolver::_new(resolver),
        }
    }
}

impl Service<Request<Body>> for Server {
    type Response = Response;
    type Error = Error;
    type Future = Handling;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        Handling {
            inner: self.clone(),
            state: State::Initial,
            request: Some(request),
        }
    }
}

/// Future that will drive the handling of the request.
struct Handling {
    inner: Server,
    state: State,
    request: Option<Request<Body>>,
}

enum State {
    Initial,
    Resolve(Option<ResolutionFuture>),
    Ready(GatewayService),
}

impl Future for Handling {
    type Output = Result<Response>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // TODO: Return better errors.
        let this = &mut *self;
        let request = this.request.take().expect("There to be a request");
        let cid = request.uri().path().trim_start_matches('/').to_string();
        loop {
            let next = match this.state {
                State::Initial => {
                    if let Some(mut svc) = this.inner.cache.get(&cid) {
                        return Pin::new(&mut svc.call(request))
                            .poll(cx)
                            .map_err(Error::msg);
                    }
                    ready!(this.inner.resolver.0.poll_ready(cx))?;
                    State::Resolve(Some(this.inner.resolver.0.call(cid.clone())))
                }
                State::Resolve(ref mut resolving) => {
                    let mut resolving = resolving.take().expect("Future");
                    let svc = ready!(Pin::new(&mut resolving).poll(cx))?;
                    // TODO: Make bound configurable.
                    let svc = Worker::new(svc, 10000);
                    this.inner.cache.insert(cid.clone(), svc.clone());
                    State::Ready(svc)
                }
                State::Ready(ref mut svc) => {
                    ready!(svc.poll_ready(cx)).map_err(Error::msg)?;
                    return Pin::new(&mut svc.call(request))
                        .poll(cx)
                        .map_err(Error::msg);
                }
            };
            this.state = next;
        }
    }
}
