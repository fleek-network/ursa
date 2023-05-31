use crate::{
    resolve::{cid::Cid, Key},
    types::{Client, Worker},
};
use anyhow::{Error, Result};
use axum::http::Request;
use futures::Stream;
use hyper::Body;
use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tower::{
    balance::p2c::MakeBalance,
    discover::Change,
    load::{CompleteOnResponse, PeakEwmaDiscover},
    {BoxError, Service},
};

/// [`tower::discover::Discover`] that returns sets of backend services. See [`tower::balance::p2c::MakeBalance`].
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

// TODO: Make these configurable.
impl Default for PeakEwmaConfig {
    fn default() -> Self {
        Self {
            default_rtt: Duration::from_millis(30),
            decay: Duration::from_secs(10),
            completion: Default::default(),
        }
    }
}

/// Wrapper around a generic CID resolver.
// TODO: Maybe we could bound the response so that
// it's more clear the service that R provides.
pub struct Resolve<R>
where
    R: Service<Cid>,
    <R as Service<Cid>>::Error: Into<BoxError>,
{
    client: Client,
    cid_resolver: Worker<R, Cid>,
    config: Arc<Config>,
}

impl<R> Clone for Resolve<R>
where
    R: Service<Cid>,
    <R as Service<Cid>>::Error: Into<BoxError>,
{
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            config: self.config.clone(),
            cid_resolver: self.cid_resolver.clone(),
        }
    }
}

impl<R> Resolve<R>
where
    R: Service<Cid>,
    <R as Service<Cid>>::Error: Into<BoxError>,
{
    pub fn new(indexer: Worker<R, Cid>, config: Arc<Config>) -> Self {
        Self {
            client: Client::new(),
            config,
            cid_resolver: indexer,
        }
    }
}

impl<R, S, Req> Service<Cid> for Resolve<R>
where
    R: Service<Cid, Response = Cluster<S, Req>> + Clone + Unpin + 'static,
    S: Service<Req> + Clone + Unpin + 'static,
    <R as Service<Cid>>::Future: Send,
    <R as Service<Cid>>::Error: Into<BoxError>,
    Req: Unpin + 'static,
{
    type Response = PeakEwmaDiscover<Cluster<S, Req>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.cid_resolver.poll_ready(cx).map_err(Error::msg)
    }

    fn call(&mut self, cid: Cid) -> Self::Future {
        tracing::trace!("Resolving {cid:?}");
        let config = self.config.clone();
        let resolver = self.cid_resolver.clone();
        let mut resolver = std::mem::replace(&mut self.cid_resolver, resolver);
        let fut = async move {
            let cluster_svc = resolver.call(cid).await.map_err(Error::msg)?;
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

// TODO: Make inner type private. Also do we need this wrapper? Let's try to simplify.
/// Wrapper to abstract Balance type.
#[derive(Clone)]
pub struct Resolver<R>(pub MakeBalance<Resolve<R>, Request<Body>>)
where
    R: Service<Cid>,
    <R as Service<Cid>>::Error: Into<BoxError> + Send + Sync,
    <R as Service<Cid>>::Future: Send;

impl<R> Resolver<R>
where
    R: Service<Cid> + Send + 'static,
    <R as Service<Cid>>::Error: Into<BoxError> + Send + Sync,
    <R as Service<Cid>>::Future: Send,
{
    pub fn new(resolver: R) -> Self {
        Self(MakeBalance::new(Resolve::new(
            // TODO: Make bound configurable.
            Worker::new(resolver, 10000),
            Arc::new(Config::default()),
        )))
    }
}

#[test]
mod test {
    use crate::resolve::{
        cid::Cid,
        resolver::{Cluster, Config, Resolve},
    };
    use crate::types::Worker;
    use anyhow::{Error, Result};
    use hyper::body::HttpBody;
    use hyper::{Body, Request, Response};
    use std::collections::HashMap;
    use std::net::IpAddr;
    use std::str::FromStr;
    use std::sync::Arc;
    use std::task::{Context, Poll};
    use tower::Service;

    #[derive(Clone, Debug)]
    struct MockBackend(IpAddr);

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

    impl Service<Cid> for MockIndexer {
        type Response = Cluster<MockBackend, Request<Body>>;
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

        fn call(&mut self, cid: Cid) -> Self::Future {
            let backend = self.0.get(&cid).unwrap().clone();
            let fut = async move { Ok(Cluster::new(vec![(backend.0, backend)])) };
            return Box::pin(fut);
        }
    }

    #[tokio::test]
    async fn test_resolve() {
        // Given: Some CIDs.
        let cid1 = "cid1".to_string();
        let cid2 = "cid2".to_string();
        let svc1_address = IpAddr::from_str("192.0.0.1").unwrap();
        let svc2_address = IpAddr::from_str("192.0.0.2").unwrap();

        // Given: Mock Indexer that dynamically returns sets of services given a cid.
        // Given: Some mock backends that will return their address.
        // Given: The resolver.
        let mut services = HashMap::new();
        services.insert(cid1.clone(), MockBackend(svc1_address));
        services.insert(cid2.clone(), MockBackend(svc2_address));
        let resolver = Resolve::new(
            Worker::new(MockIndexer(services), 10),
            Arc::new(Config::default()),
        );
        let mut svc: tower::balance::p2c::MakeBalance<Resolve<MockIndexer>, Request<Body>> =
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
