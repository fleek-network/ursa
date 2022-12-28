use anyhow::{bail, Result};
use hyper::{body, client::HttpConnector, Body, Uri};
use hyper_tls::HttpsConnector;
use serde_json::from_slice;
use tracing::{debug, error};

type Client = hyper::client::Client<HttpsConnector<HttpConnector>, Body>;

use self::model::IndexerResponse;

pub mod model;

pub struct Indexer {
    cid_url: String,
    client: Client,
}

impl Indexer {
    pub fn new(cid_url: String, client: Client) -> Self {
        Self { cid_url, client }
    }

    pub async fn query(&self, cid: String) -> Result<IndexerResponse> {
        let endpoint = format!("{}/{cid}", self.cid_url);
        let uri = match endpoint.parse::<Uri>() {
            Ok(uri) => uri,
            Err(e) => {
                error!("error parsed uri: {}\n{}", endpoint, e);
                bail!("error parsed uri: {endpoint}")
            }
        };

        let resp = match self.client.get(uri).await {
            Ok(resp) => resp,
            Err(e) => {
                error!("error requested uri: {}\n{}", endpoint, e);
                bail!("error requested uri: {endpoint}")
            }
        };

        let bytes = match body::to_bytes(resp.into_body()).await {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("error read data from upstream: {}\n{}", endpoint, e);
                bail!("error read data from upstream: {endpoint}")
            }
        };

        let indexer_response: IndexerResponse = match from_slice(&bytes) {
            Ok(indexer_response) => indexer_response,
            Err(e) => {
                error!("error parsed indexer response from upstream: {endpoint}\n{e}");
                bail!("error parsed indexer response from upstream: {endpoint}")
            }
        };

        debug!("received indexer response for {cid}:\n{indexer_response:?}");

        Ok(indexer_response)

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
}
