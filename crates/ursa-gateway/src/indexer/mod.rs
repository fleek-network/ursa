mod model;

use crate::util::Client;
use anyhow::{Error, Result};
use futures::Stream;
use hyper::{Body, Request, Response, Uri};
use std::{
    future::Future,
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

impl Cluster<Handler> {
    pub fn new(client: Client) -> Cluster<Handler> {
        Self {
            services: vec![(
                0,
                Handler {
                    uri: "foo".parse().unwrap(),
                    client,
                },
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

// TODO: Implement Service.
// This will query the edge node for the content.
#[derive(Clone)]
pub struct Handler {
    uri: Uri,
    client: Client,
}

impl Service<Request<Body>> for Handler {
    type Response = Response<Body>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response>>>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: Request<Body>) -> Self::Future {
        let this = self.clone();
        Box::pin(async move {
            match this.client.get(this.uri).await {
                Ok(response) => Ok(response),
                Err(e) => Err(e.into()),
            }
        })
    }
}
