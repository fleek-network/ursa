mod model;

use anyhow::{Error, Result};
use futures::Stream;
use hyper::{Body, Request, Response};
use std::{
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{discover::Change, Service};

type Key = SocketAddr;

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
    pub fn new(services: Vec<(Key, S)>) -> Cluster<S> {
        Self { services }
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
