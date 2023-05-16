mod model;

use anyhow::{Error, Result};
use futures::Stream;
use hyper::{Body, Request, Response};
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tower::{discover::Change, Service};

type Key = usize;

pub enum IndexerCommand<S>
where
    S: Service<Request<Body>, Response = Response<Body>, Error = Error>,
{
    GetProviderList {
        cid: String,
        tx: tokio::sync::oneshot::Sender<Result<Cluster<S>>>,
    },
}

// TODO: This will be returned by the indexer worker.
pub struct Cluster<S> {
    services: Vec<(Key, S)>,
}

impl<S> Cluster<S> {
    pub fn new(backend: S) -> Cluster<S> {
        Self {
            services: vec![(
                0, // TODO: Remove unwrap.
                backend,
            )],
        }
    }
}

impl<S> Stream for Cluster<S>
where
    S: Service<Request<Body>, Response = Response<Body>, Error = Error> + Unpin,
{
    type Item = Result<Change<Key, S>, Error>;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.get_mut().services.pop() {
            Some((k, service)) => Poll::Ready(Some(Ok(Change::Insert(k, service)))),
            None => Poll::Pending,
        }
    }
}
