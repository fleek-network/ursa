pub mod model;

use std::net::SocketAddrV4;

use anyhow::{bail, Context, Result};
use hyper::{body, client::HttpConnector, Body, Request, Uri};
use hyper_tls::HttpsConnector;
use jsonrpc_v2::{Id, V2};
use libp2p::multiaddr::Protocol;
use model::IndexerResponse;
use serde::Deserialize;
use serde_json::{from_slice, json};
use tracing::{debug, error};

type Client = hyper::client::Client<HttpsConnector<HttpConnector>, Body>;

pub struct Resolver {
    indexer_cid_url: String,
    client: Client,
}

impl Resolver {
    pub fn new(indexer_cid_url: String, client: Client) -> Self {
        Self {
            indexer_cid_url,
            client,
        }
    }

    pub async fn provider_address_v4(&self, cid: &str) -> Result<Vec<SocketAddrV4>> {
        let endpoint = format!("{}/{cid}", self.indexer_cid_url);
        let uri = match endpoint.parse::<Uri>() {
            Ok(uri) => uri,
            Err(e) => {
                error!("Error parsed uri: {endpoint} {e:?}");
                bail!("Error parsed uri: {endpoint}")
            }
        };

        let resp = match self.client.get(uri).await {
            Ok(resp) => resp,
            Err(e) => {
                error!("Error requested uri: {endpoint} {e:?}");
                bail!("Error requested uri: {endpoint}")
            }
        };

        let bytes = match body::to_bytes(resp.into_body()).await {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Error read data from upstream: {endpoint} {e:?}");
                bail!("Error read data from upstream: {endpoint}")
            }
        };

        let indexer_response: IndexerResponse = match from_slice(&bytes) {
            Ok(indexer_response) => indexer_response,
            Err(e) => {
                error!("Error parsed indexer response from upstream: {endpoint} {e:?}");
                bail!("Error parsed indexer response from upstream: {endpoint}")
            }
        };

        debug!("Received indexer response for {cid}: {indexer_response:?}");

        choose_provider(indexer_response)

        // TODO:
        // 1. filter FleekNetwork metadata
        // 2. pick node (round-robin)
        // 3. call get_block to node
        // 4.
        //   4.1 return block?
        //   4.2 resolve?
        //
        // IMPROVEMENTS:
        // 1. maintain N workers keep track of indexing data
        // 2. cherry-pick closest node
        // 3. cache TTL
    }

    pub async fn resolve_content(
        &self,
        addresses: Vec<SocketAddrV4>,
        cid: &str,
    ) -> Result<Vec<u8>> {
        for addr in addresses.into_iter() {
            let req = Request::builder()
                .method("POST")
                .uri(format!("http://{}:{}/rpc/v0", addr.ip(), addr.port()))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": "id",
                        "method": "ursa_get_cid",
                        "params":{"cid": cid}
                    })
                    .to_string(),
                ))
                .context("Request to be valid")?;

            match self.client.request(req).await {
                Ok(resp) if resp.status() == 404 => bail!("Failed to find content for {cid}"),
                Ok(resp) => {
                    let bytes = body::to_bytes(resp.into_body()).await?;
                    match from_slice::<JsonRpcResponse<Vec<u8>>>(&bytes) {
                        Ok(JsonRpcResponse::Result { result, .. }) => return Ok(result),
                        Ok(JsonRpcResponse::Error {
                            error: JsonRpcError { code, message },
                            ..
                        }) => {
                            error!("Server returned error with code {code} and message {message}");
                            bail!("Server returned error with code {code} and message {message}");
                        }
                        Err(e) => {
                            error!("Error parsed response from provider: {e:?}");
                            bail!("Error parsed response from provider");
                        }
                    }
                }
                Err(e) => error!("Error querying the node provider {addr:?} {e:?}"),
            }
        }
        bail!("Failed to get data")
    }
}

fn choose_provider(indexer_response: IndexerResponse) -> Result<Vec<SocketAddrV4>> {
    let providers = &indexer_response
        .multihash_results
        .first()
        .context("Indexer result did not contain a multi-hash result")?
        .provider_results;

    if providers.is_empty() {
        bail!("Multi-hash result did not contain a provider")
    }

    let multi_addresses = providers[0].provider.addrs.iter();

    let mut provider_addresses = Vec::new();
    for m_addr in multi_addresses {
        let mut addr_iter = m_addr.iter();
        let ip = match addr_iter.next() {
            Some(Protocol::Ip4(ip)) => ip,
            _ => {
                debug!("Skipping address {m_addr}");
                continue;
            }
        };
        let port = match addr_iter.next() {
            Some(Protocol::Tcp(port)) => port,
            _ => {
                debug!("Skipping address {m_addr} without port");
                continue;
            }
        };
        provider_addresses.push(SocketAddrV4::new(ip, port));
    }

    if provider_addresses.is_empty() {
        bail!("Failed to get a valid address for provider");
    }

    Ok(provider_addresses)
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum JsonRpcResponse<R> {
    Result {
        jsonrpc: V2,
        result: R,
        id: Id,
    },
    Error {
        jsonrpc: V2,
        error: JsonRpcError,
        id: Id,
    },
}

#[derive(Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}
