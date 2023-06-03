use crate::{
    backend::Backend,
    resolve::{CIDResolver, Cid, Cluster, Config, MakeBackend, ResolutionError, Resolve},
    types::Worker,
};
use axum::response::{IntoResponse, Response};
use hyper::{Body, Request, StatusCode};
use moka::sync::Cache;
use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::ready,
    task::{Context, Poll},
};
use thiserror::Error;
use tower::{
    balance::p2c::{Balance, MakeFuture},
    load::PeakEwmaDiscover,
    BoxError, Service,
};

type Resolving = Pin<
    Box<
        MakeFuture<
            Pin<
                Box<
                    dyn Future<
                            Output = Result<
                                PeakEwmaDiscover<Cluster<Backend, Request<Body>>>,
                                ResolutionError,
                            >,
                        > + Send,
                >,
            >,
            Request<Body>,
        >,
    >,
>;
type BackendWorker = Worker<
    Balance<PeakEwmaDiscover<Cluster<Backend, Request<Body>>>, Request<Body>>,
    Request<Body>,
>;
type Serving = Pin<Box<dyn Future<Output = Result<Response, BoxError>> + Send>>;

/// Returns a response on an error.
macro_rules! handle_err {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => return Poll::Ready(Ok(handle_error(e))),
        }
    };
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("there was an internal error")]
    Internal,
    #[error(transparent)]
    Resolution(#[from] ResolutionError),
}

/// Service that will run in Hyper/Axum.
#[derive(Clone)]
pub struct Server {
    cache: Cache<Cid, BackendWorker>,
    resolver: MakeBackend<CIDResolver>,
}

impl Server {
    pub fn new(resolver: CIDResolver, cache: Cache<Cid, BackendWorker>) -> Self {
        Self {
            cache,
            resolver: MakeBackend::new(Resolve::new(
                // TODO: Make bound configurable.
                Worker::new(resolver, 10000),
                Arc::new(Config::default()),
            )),
        }
    }
}

impl Service<Request<Body>> for Server {
    type Response = Response;
    type Error = Infallible;
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

enum State {
    Initial,
    Resolve {
        cid: Cid,
        resolving: Resolving,
    },
    Serve {
        worker: BackendWorker,
        serving: Option<Serving>,
    },
}

impl Future for Handling {
    type Output = Result<Response, Infallible>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
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
                            handle_err!(ready!(this.server.resolver.poll_ready(cx))
                                .map_err(Error::Resolution));
                            State::Resolve {
                                cid: cid.clone(),
                                resolving: Box::pin(this.server.resolver.call(cid)),
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
                    let svc =
                        handle_err!(ready!(resolving.as_mut().poll(cx)).map_err(Error::Resolution));
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
                        handle_err!(ready!(worker.poll_ready(cx)).map_err(|e| {
                            tracing::error!("backend worker failed: {e:?}");
                            Error::Internal
                        }));
                        tracing::trace!("Ready to handle the request");
                        let request = this.request.take().expect("There to be a request");
                        serving.replace(Box::pin(worker.call(request)));
                    }
                    tracing::trace!("Ready to handle the request");
                    let response = ready!(serving
                        .as_mut()
                        .expect("There to be a future")
                        .as_mut()
                        .poll(cx));

                    return match response {
                        // Tower's Balance returns Boxed errors so we have no way to
                        // propagate typed errors all the way here.
                        Err(e) => {
                            Poll::Ready(Ok(
                                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
                            ))
                        }
                        Ok(response) if !response.status().is_success() => {
                            Poll::Ready(Ok(StatusCode::BAD_GATEWAY.into_response()))
                        }
                        Ok(response) => Poll::Ready(Ok(response)),
                    };
                }
            };
            this.state = next;
        }
    }
}

fn handle_error(error: Error) -> Response {
    match error {
        Error::Internal | Error::Resolution(ResolutionError::Internal(_)) => {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Error::Resolution(ResolutionError::InvalidResponseFromIndexer) => {
            StatusCode::BAD_GATEWAY.into_response()
        }
    }
}
