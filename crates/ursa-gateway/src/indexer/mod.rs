mod model;

use anyhow::{Error, Result};
use futures::Stream;
use std::{
    marker::PhantomData,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{discover::Change, Service};

type Key = SocketAddr;

// TODO: The plan is to send the indexer commands to fetch clusters
// and commands to remove backends, from those clusters, that failed.
// Could/should we delegate the management of a cluster
// to another service that could also serve as a cache?
pub enum Request<Cid = String> {
    Get(Cid),
}

pub struct Response<S, Req>(pub Option<Cluster<S, Req>>);

// TODO: This will be returned by the indexer worker.
pub struct Cluster<S, Req> {
    services: Vec<(Key, S)>,
    _req: PhantomData<Req>,
}

impl<S, Req> Clone for Cluster<S, Req>
where
    S: Clone,
{
    fn clone(&self) -> Cluster<S, Req> {
        Self {
            services: self.services.clone(),
            _req: PhantomData,
        }
    }
}

impl<S, Req> Cluster<S, Req> {
    pub fn new(services: Vec<(Key, S)>) -> Cluster<S, Req> {
        Self {
            services,
            _req: Default::default(),
        }
    }
}

impl<S, Req> Stream for Cluster<S, Req>
where
    S: Service<Req> + Unpin,
    Req: Unpin,
{
    type Item = Result<Change<Key, S>, Error>;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.get_mut().services.pop() {
            Some((k, service)) => Poll::Ready(Some(Ok(Change::Insert(k, service)))),
            None => Poll::Pending,
        }
    }
}
