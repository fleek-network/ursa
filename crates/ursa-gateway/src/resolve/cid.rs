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

#[derive(Clone)]
pub struct CIDResolver {
    inner: Arc<State>,
}

pub struct State {
    indexer_url: String,
    client: Client,
}

impl Service<Cid> for CIDResolver {
    type Response = Cluster<Backend, HttpRequest<Body>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response>>>>;

    #[inline]
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, cid: Cid) -> Self::Future {
        let state = self.inner.clone();
        let fut = async move {
            // Resolve.
            let uri = format!("{}/{:?}", state.indexer_url, cid)
                .parse::<Uri>()
                .map_err(Error::msg)?;
            let response = state.client.get(uri).await?;
            if response.status() != StatusCode::OK {
                return Err(anyhow!(
                    "Bad response from the indexer {}",
                    state.indexer_url
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
            Ok(Cluster::new(services))
        };
        Box::pin(fut)
    }
}
