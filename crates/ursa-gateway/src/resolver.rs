use crate::{
    indexer::{Cluster, IndexerCommand},
    util::Client,
};
use anyhow::{Error, Result};
use axum::http::Request;
use hyper::{Body, Response};
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tokio::sync::mpsc::Sender;
use tower::{
    load::{CompleteOnResponse, PeakEwmaDiscover},
    Service,
};
use tracing::error;

pub type Cid = String;

struct PeakEwmaConfig {
    default_rtt: Duration,
    decay: Duration,
    completion: CompleteOnResponse,
}

impl Default for PeakEwmaConfig {
    fn default() -> Self {
        Self {
            default_rtt: Duration::from_millis(30),
            decay: Duration::from_secs(10),
            completion: Default::default(),
        }
    }
}

/// Reads the cluster identifier (cid) from the request
/// and returns a set of services (cluster) wrapped by a Balance.
#[derive(Clone)]
pub struct Resolver<S>
where
    S: Service<Request<Body>, Response = Response<Body>, Error = Error> + Clone + Unpin + 'static,
{
    client: Client,
    indexer_tx: Sender<IndexerCommand<S>>,
    indexer_cid_url: String,
    config: Arc<PeakEwmaConfig>,
}

impl<S> Resolver<S>
where
    S: Service<Request<Body>, Response = Response<Body>, Error = Error> + Clone + Unpin + 'static,
{
    pub fn new(indexer_tx: Sender<IndexerCommand<S>>) -> Self {
        Self {
            client: Client::new(),
            indexer_cid_url: String::new(),
            indexer_tx,
            // TODO: Pass this as argument.
            config: Arc::new(Default::default()),
        }
    }
}

impl<S> Service<Cid> for Resolver<S>
where
    S: Service<Request<Body>, Response = Response<Body>, Error = Error> + Clone + Unpin + 'static,
{
    type Response = PeakEwmaDiscover<Cluster<S>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response>>>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, cid: Cid) -> Self::Future {
        let this = self.clone();
        let fut = async move {
            let (tx, rx) = tokio::sync::oneshot::channel();
            // Send request to indexer worker.
            let cmd = IndexerCommand::GetProviderList { cid, tx };
            if let Err(e) = this.indexer_tx.send(cmd).await {
                error!("Sending failed {e}");
                return Err(anyhow::anyhow!("Sending failed {e}"));
            }
            let l = rx.await??;
            Ok(PeakEwmaDiscover::new(
                l,
                this.config.default_rtt,
                this.config.decay,
                this.config.completion,
            ))
        };
        Box::pin(fut)
    }
}

mod test {
    use crate::resolver::Resolver;
    use anyhow::{Error, Result};

    #[test]
    fn test_load_balancer() {
        #[derive(Clone)]
        struct Mock;

        impl tower::Service<hyper::Request<hyper::Body>> for Mock {
            type Response = hyper::Response<hyper::Body>;
            type Error = Error;
            type Future =
                std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response>>>>;

            fn poll_ready(
                &mut self,
                _: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Result<(), Self::Error>> {
                std::task::Poll::Ready(Ok(()))
            }

            fn call(&mut self, _: hyper::Request<hyper::Body>) -> Self::Future {
                Box::pin(async { Ok(hyper::Response::new(hyper::Body::empty())) })
            }
        }

        let (tx, _) = tokio::sync::mpsc::channel(100000);
        let resolver = Resolver::new(tx);
        let _: tower::balance::p2c::MakeBalance<Resolver<Mock>, hyper::Request<hyper::Body>> =
            tower::balance::p2c::MakeBalance::new(resolver);
        println!("Hello world!")
    }
}
