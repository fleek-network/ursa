pub mod model;

use anyhow::{anyhow, Context};
use axum::{
    body::Body,
    http::response::{Parts, Response},
};
use geoutils::Location;
use hyper::{
    body::to_bytes,
    client::{self, HttpConnector},
    StatusCode, Uri,
};
use hyper_tls::HttpsConnector;
use libp2p::multiaddr::Protocol;
use maxminddb::{geoip2::City, Reader};
use model::IndexerResponse;
use moka::sync::Cache;
use serde_json::from_slice;
use std::{net::IpAddr, sync::Arc};
use tracing::{debug, error, info, warn};

use crate::{
    picker::model::{Metadata, MultihashResult, ProviderResult},
    util::error::Error,
};

const FLEEK_NETWORK_FILTER: &[u8] = b"FleekNetwork";

type Client = client::Client<HttpsConnector<HttpConnector>, Body>;

pub struct Picker {
    indexer_cid_url: String,
    client: Client,
    cache: Cache<String, Vec<MultihashResult>>,
    maxminddb: Arc<Reader<Vec<u8>>>,
    location: Location,
}

impl Picker {
    pub fn new(
        indexer_cid_url: String,
        client: Client,
        cache: Cache<String, Vec<MultihashResult>>,
        maxminddb: Arc<Reader<Vec<u8>>>,
        addr: IpAddr,
    ) -> Result<Self, Error> {
        let city = maxminddb
            .lookup::<City>(addr)
            .map_err(|e| anyhow!(e.to_string()))?;
        let location = get_location(city)?;
        Ok(Self {
            indexer_cid_url,
            client,
            cache,
            maxminddb,
            location,
        })
    }

    fn distance(&self, ip: IpAddr) {
        debug!(
            "Distance to {ip:?} is {:?}",
            self.maxminddb.lookup::<City>(ip).unwrap()
        )
    }

    pub async fn resolve_content(&self, cid: &str) -> Result<Response<Body>, Error> {
        let endpoint = format!("{}/{cid}", self.indexer_cid_url);

        let uri = endpoint.parse::<Uri>().map_err(|e| {
            error!("Error parsed uri: {endpoint} {e:?}");
            anyhow!("Error parsed uri: {endpoint}")
        })?;

        let multihash_results = match self.cache.get(cid) {
            None => {
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
                    (parts, body) => {
                        error!("Error requested indexer {endpoint} with parts {parts:?} and body {body:?}");
                        return Err(Error::Upstream(
                            parts.status,
                            format!("Error requested indexer: {endpoint}"),
                        ));
                    }
                };

                let bytes = to_bytes(body).await.map_err(|e| {
                    error!("Error read data from indexer: {endpoint} {e:?}");
                    anyhow!("Error read data from indexer {endpoint}")
                })?;

                let indexer_response: IndexerResponse = from_slice(&bytes).map_err(|e| {
                    error!("Error parsed indexer response from indexer: {endpoint} {e:?}");
                    anyhow!("Error parsed indexer response from indexer: {endpoint}")
                })?;

                debug!("Received indexer response for {cid}: {indexer_response:?}");

                self.cache
                    .insert(cid.to_string(), indexer_response.multihash_results.clone());

                indexer_response.multihash_results
            }
            Some(multihash_results) => multihash_results,
        };

        let providers: Vec<(&ProviderResult, Metadata)> = multihash_results
            .first()
            .context("Indexer result did not contain a multi-hash result")?
            .provider_results
            .iter()
            .filter_map(|provider| {
                let metadata_bytes = match base64::decode(&provider.metadata) {
                    Ok(b) => b,
                    Err(e) => {
                        error!("Failed to decode metadata {e:?}");
                        return None;
                    }
                };
                let metadata = match bincode::deserialize::<Metadata>(&metadata_bytes) {
                    Ok(b) => b,
                    Err(e) => {
                        error!("Failed to deserialize metadata {e:?}");
                        return None;
                    }
                };
                if metadata.data == FLEEK_NETWORK_FILTER {
                    return Some((provider, metadata));
                }
                warn!("Invalid data in metadata {:?}", metadata.data);
                None
            })
            .collect();

        // TODO:
        // cherry-pick closest node
        let (provider, metadata) = providers
            .first() // FIXME: temporary
            .context("Multi-hash result did not contain a provider")?;

        info!("File size received {}", metadata.size);

        let provider_addresses: Vec<String> = provider
            .provider
            .addrs
            .iter()
            .map(|m_addr| {
                let (mut protocol, mut host, mut port) =
                    (String::from("http"), String::new(), String::new());
                for addr in m_addr.into_iter() {
                    match addr {
                        Protocol::Ip6(ip) => {
                            // TODO: Remove.
                            self.distance(ip.into());
                            host = ip.to_string();
                        }
                        Protocol::Ip4(ip) => {
                            // TODO: Remove.
                            self.distance(ip.into());
                            host = ip.to_string();
                        }
                        Protocol::Tcp(p) => {
                            port = p.to_string();
                        }
                        Protocol::Https => {
                            protocol = "https".to_string();
                        }
                        _ => {}
                    };
                }
                (
                    format!("{protocol}://{host}:{port}"),
                    host.is_empty() || port.is_empty(),
                )
            })
            .filter(|(_, incomplete)| !incomplete)
            .map(|(addr, _)| addr)
            .collect();

        if provider_addresses.is_empty() {
            return Err(Error::Internal(
                "Failed to get a valid address for provider".to_string(),
            ));
        }

        debug!("Provider addresses to query: {provider_addresses:?}");

        for addr in provider_addresses.into_iter() {
            let endpoint = format!("{addr}/ursa/v0/{cid}");
            let uri = match endpoint.parse::<Uri>() {
                Ok(uri) => uri,
                Err(e) => {
                    error!("Error parsed uri: {endpoint} {e:?}");
                    continue;
                }
            };
            match self.client.get(uri).await {
                Ok(resp) => {
                    return Ok(resp);
                }
                Err(e) => error!("Error querying the node provider: {endpoint:?} {e:?}"),
            };
        }

        Err(Error::Internal("Failed to get data".to_string()))
    }
}

fn get_location(city: City) -> Result<Location, Error> {
    let location = city.location.ok_or_else(|| anyhow!("missing location"))?;
    let latitude = location
        .latitude
        .ok_or_else(|| anyhow!("missing latitude"))?;
    let longitude = location
        .longitude
        .ok_or_else(|| anyhow!("missing longitude"))?;
    Ok(Location::new(latitude, longitude))
}
