use crate::{
    backend::Backend,
    resolve::{
        indexer::model::{IndexerResponse, Metadata, ProviderResult},
        resolver::Cluster,
        Key, FLEEK_NETWORK_FILTER,
    },
    types::Client,
};
use anyhow::{anyhow, Context as AnyhowContext, Error, Result};
use hyper::{body::to_bytes, Body, Request as HttpRequest, StatusCode, Uri};
use libp2p::multiaddr::Protocol;
use serde_json::from_slice;
use std::{
    future::Future,
    net::IpAddr,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tower::Service;
use tracing::{error, warn};

pub type Cid = String;

pub struct Config {
    indexer_url: String,
}

// TODO: The plan is to send the indexer commands to fetch clusters
// and commands to remove backends, from those clusters, that failed.
// Could/should we delegate the management of a cluster
// to another service that could also serve as a cache?
pub enum Request<Cid = String> {
    Get(Cid),
}

pub struct Response<S, Req>(pub Option<Cluster<S, Req>>);

pub struct Indexer {
    inner: Arc<State>,
}

pub struct State {
    config: Config,
    client: Client,
}

impl Service<Request<Cid>> for Indexer {
    type Response = Response<Backend, HttpRequest<Body>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response>>>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Cid>) -> Self::Future {
        let state = self.inner.clone();
        let fut = async move {
            let Request::Get(cid) = req;
            // Resolve.
            let uri = format!("{}/{:?}", state.config.indexer_url, cid)
                .parse::<Uri>()
                .map_err(Error::msg)?;
            let response = state.client.get(uri).await?;
            if response.status() != StatusCode::OK {
                return Err(anyhow!(
                    "Bad response from the indexer {}",
                    state.config.indexer_url
                ));
            }
            let body = response.into_body();
            let bytes = to_bytes(body).await.map_err(Error::msg)?;
            let indexer_response: IndexerResponse =
                from_slice(&bytes).context("Error parsed indexer response from indexer")?;
            let result: Vec<&ProviderResult> = indexer_response
                .multihash_results
                .first()
                .context("Indexer result did not contain a multi-hash result")?
                .provider_results
                .iter()
                .filter(|provider| {
                    let metadata_bytes = match base64::decode(&provider.metadata) {
                        Ok(b) => b,
                        Err(e) => {
                            error!("Failed to decode metadata {e:?}");
                            return false;
                        }
                    };
                    let metadata = match bincode::deserialize::<Metadata>(&metadata_bytes) {
                        Ok(b) => b,
                        Err(e) => {
                            error!("Failed to deserialize metadata {e:?}");
                            return false;
                        }
                    };
                    if metadata.data == FLEEK_NETWORK_FILTER {
                        return true;
                    }
                    warn!("Invalid data in metadata {:?}", metadata.data);
                    false
                })
                .collect();
            let services: Vec<(Key, Backend)> = result
                .into_iter()
                .flat_map(|provider_result| &provider_result.provider.addrs)
                .filter_map(|multiaddr| {
                    let (mut protocol, mut host, mut port) = (String::from("http"), None, None);
                    for addr in multiaddr.into_iter() {
                        match addr {
                            Protocol::Ip6(ip) => {
                                host = Some(IpAddr::from(ip));
                            }
                            Protocol::Ip4(ip) => {
                                host = Some(IpAddr::from(ip));
                            }
                            Protocol::Tcp(p) => {
                                port = Some(p);
                            }
                            Protocol::Https => {
                                protocol = "https".to_string();
                            }
                            _ => {}
                        };
                    }
                    if host.is_none() || port.is_none() {
                        return None;
                    }
                    let host = host.unwrap();
                    let port = port.unwrap();
                    // TODO: Remove unwrap().
                    let uri = format!("{protocol}://{host}:{port}").parse().unwrap();
                    Some((host, Backend::new(uri, state.client.clone())))
                })
                .collect();
            Ok(Response(Some(Cluster::new(services))))
        };
        Box::pin(fut)
    }
}
