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
            server: self.clone(),
            state: State::Initial,
            request: Some(request),
        }
    }
}

/// Future that will drive the handling of the request.
pub struct Handling {
    server: Server,
    state: State,
    request: Option<Request<Body>>,
}

// TODO:
// Let's save the futures in the state.
// Can we avoid boxing and just use generics?
enum State {
    Initial,
    Resolve {
        cid: Cid,
        resolving: Pin<Box<ResolutionFuture>>,
    },
    Serve {
        worker: BackendServiceWorker,
        serving:
            Option<Pin<Box<dyn Future<Output = std::result::Result<Response, BoxError>> + Send>>>,
    },
}

impl Future for Handling {
    type Output = Result<Response>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // TODO: Return better errors.
        let mut this = &mut *self;
        loop {
            let next = match &mut this.state {
                State::Initial => {
                    // TODO: We need a better way to get the cid from a request.
                    // Maybe we can introduce a layer that parses this from path
                    // or from Host header.
                    // Axum extensions does not currently return this for us
                    // https://github.com/tokio-rs/axum/issues/2029.
                    // At the moment, this computation keep calculating cid
                    // everytime poll_ready returns pending.
                    let cid = this
                        .request
                        .as_ref()
                        .map(|request| request.uri().path().trim_start_matches('/').to_string())
                        .expect("There to be a request");
                    match this.server.cache.get(&cid) {
                        None => {
                            tracing::trace!("Backend cache miss");
                            ready!(this.server.resolver.0.poll_ready(cx))?;
                            State::Resolve {
                                cid: cid.clone(),
                                resolving: Box::pin(this.server.resolver.0.call(cid)),
                            }
                        }
                        Some(svc) => {
                            tracing::trace!("Backend cache hit");
                            State::Serve {
                                worker: svc,
                                serving: None,
                            }
                        }
                    }
                }
                State::Resolve { cid, resolving } => {
                    let svc = ready!(resolving.as_mut().poll(cx))?;
                    // TODO: Make bound configurable.
                    let svc = Worker::new(svc, 10000);
                    this.server.cache.insert(cid.clone(), svc.clone());
                    State::Serve {
                        worker: svc,
                        serving: None,
                    }
                }
                State::Serve { worker, serving } => {
                    if serving.is_none() {
                        ready!(worker.poll_ready(cx)).map_err(Error::msg)?;
                        tracing::trace!("Ready to handle the request");

                        let request = this.request.take().expect("There to be a request");
                        serving.replace(Box::pin(worker.call(request)));
                    }

                    tracing::trace!("Ready to handle the request");
                    return serving
                        .as_mut()
                        .expect("There to be a future")
                        .as_mut()
                        .poll(cx)
                        .map_err(Error::msg);
                }
            };
            this.state = next;
        }
    }
}
