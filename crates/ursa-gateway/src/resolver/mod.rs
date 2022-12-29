pub mod model;

use anyhow::{anyhow, bail, Context, Result};
use axum::{http::response::Parts, response::Response};
use hyper::{body, client::HttpConnector, Body, StatusCode, Uri};
use hyper_tls::HttpsConnector;
use jsonrpc_v2::{Id, V2};
use libp2p::multiaddr::Protocol;
use model::IndexerResponse;
use serde::Deserialize;
use serde_json::from_slice;
use tracing::{debug, error};

// Base64 encoded. See ursa-index-provider::Metadata.
const ENCODED_METADATA: &str = "AAkAAAAAAAAAAAAAAAAAAAwAAAAAAAAARmxlZWtOZXR3b3Jr";

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

    pub async fn provider_address_v4(&self, cid: &str) -> Result<Vec<String>> {
        let endpoint = format!("{}/{cid}", self.indexer_cid_url);

        let uri = endpoint.parse::<Uri>().map_err(|e| {
            error!("Error parsed uri: {endpoint} {e:?}");
            anyhow!("Error parsed uri: {endpoint}")
        })?;

        let body = match self
            .client
            .get(uri)
            .await
            .map_err(|e| {
                error!("Error requested indexer: {endpoint} {e:?}");
                anyhow!("Error requested indexer: {endpoint}")
            })?
            .into_parts()
        {
            (
                Parts {
                    status: StatusCode::OK,
                    ..
                },
                body,
            ) => body,
            (
                Parts {
                    status: StatusCode::NOT_FOUND,
                    ..
                },
                ..,
            ) => {
                error!("Error requested indexer - Got 404: {endpoint}");
                bail!("Error requested indexer - Got 404: {endpoint}")
            }
            resp => {
                error!("Error requested indexer: {endpoint} {resp:?}");
                bail!("Error requested indexer: {endpoint}")
            }
        };

        let bytes = body::to_bytes(body).await.map_err(|e| {
            error!("Error read data from indexer: {endpoint} {e:?}");
            anyhow!("Error read data from indexer {endpoint}")
        })?;

        let indexer_response: IndexerResponse = from_slice(&bytes).map_err(|e| {
            error!("Error parsed indexer response from indexer: {endpoint} {e:?}");
            anyhow!("Error parsed indexer response from indexer: {endpoint}")
        })?;

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
        addresses: Vec<String>,
        cid: &str,
    ) -> Result<Response<Body>> {
        for addr in addresses.into_iter() {
            let endpoint = format!("{addr}/{cid}");

            let uri = match endpoint.parse::<Uri>() {
                Ok(uri) => uri,
                Err(e) => error!("Error parsed uri: {endpoint} {e:?}"),
            };

            match self.client.get(uri).await {
                Ok(resp) => return Ok(resp),
                Err(e) => error!("Error querying the node provider {addr:?} {e:?}"),
            };
        }
        bail!("Failed to get data")
    }
}

fn choose_provider(indexer_response: IndexerResponse) -> Result<Vec<String>> {
    let providers = &indexer_response
        .multihash_results
        .first()
        .context("Indexer result did not contain a multi-hash result")?
        .provider_results;

    let provider = providers
        .first()
        .context("Multi-hash result did not contain a provider")?;

    if provider.metadata != ENCODED_METADATA {
        error!("Invalid metadata received {}", &provider.metadata);
        bail!("Invalid metadata")
    }

    let provider_addresses: Vec<String> = providers[0]
        .provider
        .addrs
        .iter()
        .map(|m_addr| {
            let (mut protocol, mut host, mut port) = (String::new(), String::new(), String::new());
            for addr in m_addr.into_iter() {
                match addr {
                    Protocol::Ip6(ip) => {
                        host = ip.to_string();
                    }
                    Protocol::Ip4(ip) => {
                        host = ip.to_string();
                    }
                    Protocol::Tcp(p) => {
                        port = p.to_string();
                    }
                    Protocol::Http => {
                        protocol = "http".to_string();
                    }
                    Protocol::Https => {
                        protocol = "https".to_string();
                    }
                    _ => {}
                };
            }
            format!("{protocol}://{host}:{port}")
        })
        .filter(|addr| addr != "://:")
        .collect();

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
