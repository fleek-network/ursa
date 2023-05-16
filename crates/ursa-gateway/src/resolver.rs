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

#[derive(Default)]
pub struct Config {
    _indexer_cid_url: String,
    load_balancer_config: PeakEwmaConfig,
}

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
    config: Arc<Config>,
}

impl<S> Resolver<S>
where
    S: Service<Request<Body>, Response = Response<Body>, Error = Error> + Clone + Unpin + 'static,
{
    pub fn new(indexer_tx: Sender<IndexerCommand<S>>, config: Arc<Config>) -> Self {
        Self {
            client: Client::new(),
            indexer_tx,
            config,
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
                this.config.load_balancer_config.default_rtt,
                this.config.load_balancer_config.decay,
                this.config.load_balancer_config.completion,
            ))
        };
        Box::pin(fut)
    }
}

mod test {
    use crate::indexer::{Cluster, IndexerCommand};
    use crate::resolver::{Cid, Config, Resolver};
    use anyhow::{Error, Result};
    use hyper::body::HttpBody;
    use hyper::{Body, Request, Response};
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::str::FromStr;
    use std::sync::Arc;
    use std::task::Poll;
    use tokio::sync::mpsc::Receiver;
    use tower::Service;

    #[derive(Clone, Debug)]
    struct MockBackend(SocketAddr);

    impl Service<Request<Body>> for MockBackend {
        type Response = Response<Body>;
        type Error = Error;
        type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response>>>>;

        fn poll_ready(&mut self, _: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _: Request<Body>) -> Self::Future {
            let inner = self.0;
            Box::pin(async move { Ok(Response::new(inner.to_string().into())) })
        }
    }

    async fn start_mock_indexer(
        services: HashMap<Cid, MockBackend>,
        mut rx: Receiver<IndexerCommand<MockBackend>>,
    ) {
        loop {
            if let Some(IndexerCommand::GetProviderList { tx, cid }) = rx.recv().await {
                let backend = services.get(&cid).unwrap().clone();
                if tx
                    .send(Ok(Cluster::new(vec![(backend.0, backend)])))
                    .is_err()
                {
                    panic!("Failed to send")
                }
            }
        }
    }

    #[tokio::test]
    async fn test_resolve() {
        // Given: Some cids.
        let cid1 = "cid1".to_string();
        let cid2 = "cid2".to_string();
        let svc1_address = SocketAddr::from_str("192.0.0.1:80").unwrap();
        let svc2_address = SocketAddr::from_str("192.0.0.2:80").unwrap();

        // Given: The resolver.
        let (tx, rx) = tokio::sync::mpsc::channel(100000);
        let resolver = Resolver::new(tx, Arc::new(Config::default()));
        let mut svc: tower::balance::p2c::MakeBalance<Resolver<MockBackend>, Request<Body>> =
            tower::balance::p2c::MakeBalance::new(resolver);

        // Given: Indexer that dynamically returns sets of services given a cid.
        // Given: Some mock backends that will return their address.
        let mut services = HashMap::new();
        services.insert(cid1.clone(), MockBackend(svc1_address));
        services.insert(cid2.clone(), MockBackend(svc2_address));

        tokio::spawn(async move { start_mock_indexer(services, rx).await });

        // When: We resolve a CID.
        let mut b = svc.call(cid1).await.unwrap();
        assert!(!tokio_test::assert_ready!(
            tokio_test::task::spawn(()).enter(|cx, _| b.poll_ready(cx))
        )
        .is_err());
        // Then: The service that handles requests with those CIDs is used.
        let response = b.call(Request::new(Body::empty())).await.unwrap();
        assert_eq!(
            response.into_body().data().await.unwrap().unwrap(),
            svc1_address.to_string()
        );

        // When: We resolve the CID.
        let mut b = svc.call(cid2).await.unwrap();
        assert!(!tokio_test::assert_ready!(
            tokio_test::task::spawn(()).enter(|cx, _| b.poll_ready(cx))
        )
        .is_err());
        // Then: The service that handles requests with those CIDs is used.
        let response = b.call(Request::new(Body::empty())).await.unwrap();
        assert_eq!(
            response.into_body().data().await.unwrap().unwrap(),
            svc2_address.to_string()
        );
    }
}
