use crate::{
    backend::Backend,
    resolve::{
        indexer::model::{IndexerResponse, Metadata, ProviderResult},
        resolver::Cluster,
        Key, ResolutionError,
    },
    types::Client,
};
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

pub type Cid = String;

const FLEEK_NETWORK_FILTER: &[u8] = b"FleekNetwork";

pub struct State {
    indexer_url: String,
    client: Client,
}

/// Resolves CIDs to sets of backend services by making queries over the network to an indexer.
#[derive(Clone)]
pub struct CIDResolver {
    inner: Arc<State>,
}

impl CIDResolver {
    pub fn new(indexer_url: String, client: Client) -> Self {
        Self {
            inner: Arc::new(State {
                indexer_url,
                client,
            }),
        }
    }
}

impl Service<Cid> for CIDResolver {
    type Response = Cluster<Backend, HttpRequest<Body>>;
    type Error = ResolutionError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    #[inline]
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, cid: Cid) -> Self::Future {
        let state = self.inner.clone();
        let fut = async move {
            let uri = format!("{}/{}", state.indexer_url, cid)
                .parse::<Uri>()
                .map_err(|e| ResolutionError::Internal(e.to_string()))?;
            tracing::trace!("Requesting providers from indexer at {uri:?}");
            let response = state
                .client
                .get(uri)
                .await
                .map_err(|e| ResolutionError::Internal(e.to_string()))?;
            if response.status() != StatusCode::OK {
                tracing::error!("Bad response from the indexer {:?}", response);
                return Err(ResolutionError::InvalidResponseFromIndexer);
            }

            let body = response.into_body();
            let bytes = to_bytes(body).await.map_err(|e| {
                tracing::error!("Failed to parse response from indexer");
                ResolutionError::Internal(e.to_string())
            })?;
            let indexer_response: IndexerResponse = from_slice(&bytes).map_err(|e| {
                tracing::error!("failed to deserialize indexer response: {e:?}");
                ResolutionError::Internal(e.to_string())
            })?;

            let result: Vec<&ProviderResult> = indexer_response
                .multihash_results
                .first()
                .ok_or_else(|| {
                    ResolutionError::Internal(
                        "Indexer result did not contain a multi-hash result".to_string(),
                    )
                })?
                .provider_results
                .iter()
                .filter(|provider| {
                    let metadata_bytes = match base64::decode(&provider.metadata) {
                        Ok(b) => b,
                        Err(e) => {
                            tracing::error!("Failed to decode metadata {e:?}");
                            return false;
                        }
                    };
                    let metadata = match bincode::deserialize::<Metadata>(&metadata_bytes) {
                        Ok(b) => b,
                        Err(e) => {
                            tracing::error!("Failed to deserialize metadata {e:?}");
                            return false;
                        }
                    };
                    if metadata.data == FLEEK_NETWORK_FILTER {
                        return true;
                    }
                    tracing::warn!("Invalid data in metadata {:?}", metadata.data);
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
                    // TODO: Make path in endpoint configurable.
                    let uri = format!("{protocol}://{host}:{port}/ursa/v0/{cid}")
                        .parse::<Uri>()
                        .map_err(|e| {
                            tracing::error!("{e:?}");
                        })
                        .ok()?;
                    Some((host, Backend::new(uri, state.client.clone())))
                })
                .collect();
            Ok(Cluster::new(services))
        };
        Box::pin(fut)
    }
}
