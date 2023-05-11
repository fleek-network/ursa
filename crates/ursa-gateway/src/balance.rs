use axum::http::Request;
use hyper::{client::HttpConnector, Body, Client as HyperClient, Uri};
use std::{
    future::Future,
    hash::Hash,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{discover::Discover, BoxError, Service};

type Client = HyperClient<HttpConnector, Body>;

/// Reads the cluster identifier (cid) from the request
/// and returns a set of services (cluster) wrapped by a Balance.
// TODO: This needs to be Clone and Send so we can use it with axum.
#[derive(Clone)]
pub struct Resolver<D> {
    client: Client,
    // TODO: Remove. Discoverer will be created on each `call`.
    _discover: D,
    indexer_cid_url: String,
}

impl<D: Discover> Resolver<D> {
    pub fn new(_discover: D) -> Self {
        Self {
            client: Client::new(),
            _discover,
            indexer_cid_url: String::new(),
        }
    }
}

impl<D> Service<Request<Body>> for Resolver<D>
where
    D: Discover + Clone + Send,
    <D as Discover>::Key: Hash,
    <D as Discover>::Service: Service<Request<Body>>,
    <<D as Discover>::Service as Service<Request<Body>>>::Error: Into<BoxError>,
{
    type Response = D;
    type Error = ();
    type Future = Pin<Box<dyn Future<Output = Result<D, ()>>>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    // TODO: Handle errors.
    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut this = self.clone();
        let fut = async move {
            // Extract cid from request.
            let cid = req.uri().path().trim_start_matches('/');
            let endpoint = format!("{}/{cid}", this.indexer_cid_url.as_str());
            // Call indexer.
            // TODO: Let's blackbox the parsing we do to the indexer response.
            let mut req = Request::default();
            *req.uri_mut() = endpoint.parse::<Uri>().unwrap();
            let _res = match this.client.call(req).await {
                Ok(r) => r,
                Err(e) => {
                    // TODO: Remove print. Handle logging better.
                    println!("{e}");
                    return Err(());
                }
            };
            // Create cluster and wrap it with tower's load.
            Err(())
        };
        Box::pin(fut)
    }
}

mod test {
    use crate::balance::Resolver;

    #[test]
    fn test_load_balancer() {
        struct Mock;

        impl tower::Service<hyper::Request<hyper::Body>> for Mock {
            type Response = hyper::Response<hyper::Body>;
            type Error = tower::BoxError;
            type Future = std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>,
            >;

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

        let resolver = Resolver::new(tower::discover::ServiceList::new(vec![Mock].into_iter()));
        let _: tower::balance::p2c::MakeBalance<
            Resolver<tower::discover::ServiceList<std::vec::IntoIter<Mock>>>,
            hyper::Request<hyper::Body>,
        > = tower::balance::p2c::MakeBalance::new(resolver);
        println!("Hello world!")
    }
}
