use crate::{
    indexer::{Cluster, Request as IndexerRequest, Response},
    types::{Client, Worker},
};
use anyhow::{Error, Result};
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tower::{
    load::{CompleteOnResponse, PeakEwmaDiscover},
    BoxError, Service,
};

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
pub struct Resolver<I>
where
    I: Service<IndexerRequest>,
    <I as Service<IndexerRequest>>::Error: Into<BoxError>,
{
    client: Client,
    // TODO: How will we implement retry?
    indexer: Worker<I, IndexerRequest>,
    config: Arc<Config>,
}

impl<I> Clone for Resolver<I>
where
    I: Service<IndexerRequest>,
    <I as Service<IndexerRequest>>::Error: Into<BoxError>,
{
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            config: self.config.clone(),
            indexer: self.indexer.clone(),
        }
    }
}

impl<I> Resolver<I>
where
    I: Service<IndexerRequest>,
    <I as Service<IndexerRequest>>::Error: Into<BoxError>,
{
    pub fn new(indexer: Worker<I, IndexerRequest>, config: Arc<Config>) -> Self {
        Self {
            client: Client::new(),
            config,
            indexer,
        }
    }
}

impl<I, S, Req> Service<Cid> for Resolver<I>
where
    I: Service<IndexerRequest<Cid>, Response = Response<S, Req>> + Clone + Unpin + 'static,
    S: Service<Req> + Clone + Unpin + 'static,
    <I as Service<IndexerRequest<Cid>>>::Error: Into<BoxError>,
    Req: Unpin + 'static,
{
    type Response = PeakEwmaDiscover<Cluster<S, Req>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.indexer.poll_ready(cx).map_err(Error::msg)
    }

    fn call(&mut self, cid: Cid) -> Self::Future {
        let config = self.config.clone();
        let indexer = self.indexer.clone();
        let mut indexer = std::mem::replace(&mut self.indexer, indexer);
        let fut = async move {
            let cluster_svc = indexer
                .call(IndexerRequest::Get(cid))
                .await
                .map_err(Error::msg)?
                .0
                .expect("Indexer to return a cluster");
            Ok(PeakEwmaDiscover::new(
                cluster_svc,
                config.load_balancer_config.default_rtt,
                config.load_balancer_config.decay,
                config.load_balancer_config.completion,
            ))
        };
        Box::pin(fut)
    }
}

mod test {
    use crate::indexer::{Cluster, Request as IndexerRequest, Response as IndexerResponse};
    use crate::resolver::{Cid, Config, Resolver};
    use crate::types::Worker;
    use anyhow::{Error, Result};
    use hyper::body::HttpBody;
    use hyper::{Body, Request, Response};
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::str::FromStr;
    use std::sync::Arc;
    use std::task::{Context, Poll};
    use tower::Service;

    #[derive(Clone, Debug)]
    struct MockBackend(SocketAddr);

    impl Service<Request<Body>> for MockBackend {
        type Response = Response<Body>;
        type Error = Error;
        type Future = std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Self::Response>> + Send + 'static>,
        >;

        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _: Request<Body>) -> Self::Future {
            let inner = self.0;
            Box::pin(async move { Ok(Response::new(inner.to_string().into())) })
        }
    }

    #[derive(Clone)]
    struct MockIndexer(HashMap<Cid, MockBackend>);

    impl Service<IndexerRequest> for MockIndexer {
        type Response = IndexerResponse<MockBackend, Request<Body>>;
        type Error = Error;
        type Future = std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Self::Response>> + Send + 'static>,
        >;

        fn poll_ready(
            &mut self,
            _: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: IndexerRequest) -> Self::Future {
            let IndexerRequest::Get(cid) = req;
            let backend = self.0.get(&cid).unwrap().clone();
            let fut = async move {
                Ok(IndexerResponse(Some(Cluster::new(vec![(
                    backend.0, backend,
                )]))))
            };
            return Box::pin(fut);
        }
    }

    #[tokio::test]
    async fn test_resolve() {
        // Given: Some CIDs.
        let cid1 = "cid1".to_string();
        let cid2 = "cid2".to_string();
        let svc1_address = SocketAddr::from_str("192.0.0.1:80").unwrap();
        let svc2_address = SocketAddr::from_str("192.0.0.2:80").unwrap();

        // Given: Mock Indexer that dynamically returns sets of services given a cid.
        // Given: Some mock backends that will return their address.
        // Given: The resolver.
        let mut services = HashMap::new();
        services.insert(cid1.clone(), MockBackend(svc1_address));
        services.insert(cid2.clone(), MockBackend(svc2_address));
        let resolver = Resolver::new(
            Worker::new(MockIndexer(services), 10),
            Arc::new(Config::default()),
        );
        let mut svc: tower::balance::p2c::MakeBalance<Resolver<MockIndexer>, Request<Body>> =
            tower::balance::p2c::MakeBalance::new(resolver);

        // When: We resolve a CID.
        assert!(!tokio_test::assert_ready!(
            tokio_test::task::spawn(()).enter(|cx, _| svc.poll_ready(cx))
        )
        .is_err());
        let mut balance = svc.call(cid1).await.unwrap();
        assert!(!tokio_test::assert_ready!(
            tokio_test::task::spawn(()).enter(|cx, _| balance.poll_ready(cx))
        )
        .is_err());
        // Then: The service that handles requests with those CIDs is used.
        let response = balance.call(Request::new(Body::empty())).await.unwrap();
        assert_eq!(
            response.into_body().data().await.unwrap().unwrap(),
            svc1_address.to_string()
        );

        // When: We resolve the CID.
        assert!(!tokio_test::assert_ready!(
            tokio_test::task::spawn(()).enter(|cx, _| svc.poll_ready(cx))
        )
        .is_err());
        let mut balance = svc.call(cid2).await.unwrap();
        assert!(!tokio_test::assert_ready!(
            tokio_test::task::spawn(()).enter(|cx, _| balance.poll_ready(cx))
        )
        .is_err());
        // Then: The service that handles requests with those CIDs is used.
        let response = balance.call(Request::new(Body::empty())).await.unwrap();
        assert_eq!(
            response.into_body().data().await.unwrap().unwrap(),
            svc2_address.to_string()
        );
    }
}
