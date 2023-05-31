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
    BoxError, Service,
};

type ResolutionFuture = MakeFuture<
    Pin<Box<dyn Future<Output = Result<PeakEwmaDiscover<Cluster<Backend, Request<Body>>>>> + Send>>,
    Request<Body>,
>;
type BackendServiceWorker = Worker<
    Balance<PeakEwmaDiscover<Cluster<Backend, Request<Body>>>, Request<Body>>,
    Request<Body>,
>;

/// Service that will run in Hyper/Axum.
#[derive(Clone)]
pub struct Server {
    cache: Cache<Cid, BackendServiceWorker>,
    resolver: Resolver<CIDResolver>,
}

impl Server {
    pub fn new(resolver: CIDResolver, cache: Cache<Cid, BackendServiceWorker>) -> Self {
        Self {
            cache,
            resolver: Resolver::new(resolver),
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
        tracing::trace!("Received {request:?}");
        Handling {
            inner: self.clone(),
            state: State::Initial,
            request: Some(request),
        }
    }
}

/// Future that will drive the handling of the request.
pub struct Handling {
    inner: Server,
    state: State,
    request: Option<Request<Body>>,
}

// TODO:
// Let's save the futures in the state.
// Can we get rid of Ready?
// Can we avoid boxing and just use generics?
enum State {
    Initial,
    Resolve(Pin<Box<ResolutionFuture>>),
    Ready(BackendServiceWorker),
    Serving(Pin<Box<dyn Future<Output = std::result::Result<Response, BoxError>> + Send>>),
}

impl Future for Handling {
    type Output = Result<Response>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // TODO: Return better errors.
        let mut this = &mut *self;
        loop {
            // TODO: Get this value from header.
            let cid = "bafybeifyjj2bjhtxmp235vlfeeiy7sz6rzyx3lervfk3ap2nyn4rggqgei".to_string();
            let next = match this.state {
                State::Initial => {
                    match this.inner.cache.get(&cid) {
                        None => {
                            tracing::trace!("Backend cache miss");
                            // TODO: Poll ready does not curerntly return Pending
                            // but if it did, this future would probably move and
                            // cause the same issue from above. Fix it.
                            ready!(this.inner.resolver.0.poll_ready(cx))?;
                            State::Resolve(Box::pin(this.inner.resolver.0.call(cid.clone())))
                        }
                        Some(svc) => {
                            tracing::trace!("Backend cache hit");
                            State::Ready(svc)
                        }
                    }
                }
                State::Resolve(ref mut resolving) => {
                    // TODO: Remove these notes once you figure out what happened.
                    // Resolving future was being dropped, even after box-pinning,
                    // during poll causing Buffer's inner channel to be dropped.
                    // I think this happened because poll() is returning, so everything is getting dropped/or cloned(?)
                    // so the previous channel handle gets dropped/destroyed.
                    // let mut resolving = resolving.take().expect("Future");
                    // let mut resolving = Box::pin(resolving);
                    let svc = ready!(resolving.as_mut().poll(cx))?;
                    // TODO: Make bound configurable.
                    let svc = Worker::new(svc, 10000);
                    this.inner.cache.insert(cid.clone(), svc.clone());
                    State::Ready(svc)
                }
                State::Ready(ref mut svc) => {
                    // TODO: Poll could be called again so handle request better.
                    ready!(svc.poll_ready(cx)).map_err(Error::msg)?;
                    tracing::trace!("Ready to handle the request");

                    let request = this.request.take().expect("There to be a request");
                    State::Serving(Box::pin(svc.call(request)))
                }
                State::Serving(ref mut serving) => {
                    tracing::trace!("Ready to handle the request");
                    return serving.as_mut().poll(cx).map_err(Error::msg);
                }
            };
            this.state = next;
        }
    }
}
