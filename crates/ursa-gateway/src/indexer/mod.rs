mod model;

use crate::backend::Backend;
use crate::util::Client;
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

impl Cluster<Backend> {
    pub fn new(client: Client) -> Cluster<Backend> {
        Self {
            services: vec![(
                0,
                // TODO: Remove unwrap.
                Backend::new("foo".parse().unwrap(), client),
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
