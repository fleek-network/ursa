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
use std::cmp::Ordering;
use std::{net::IpAddr, sync::Arc};
use tracing::{debug, error, warn};

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

    fn provider_addresses(&self, providers: Vec<&ProviderResult>) -> Vec<String> {
        let mut provider_addresses = providers
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
                let city = self
                    .maxminddb
                    .lookup::<City>(host)
                    .map_err(|e| {
                        debug!(
                            "Failed to get location for ip {} with error {}",
                            host,
                            e.to_string()
                        )
                    })
                    .ok();

                let location = get_location(city?)
                    .map_err(|e| debug!("Failed to get location for city with ip {host} {:?}", e))
                    .ok()?;

                Some((location, protocol, host, port.unwrap()))
            })
            .collect::<Vec<(Location, String, IpAddr, u16)>>();
        provider_addresses.sort_by(|(location1, _, _, _), (location2, _, _, _)| {
            // Using `sorted_by` instead of `sorted_by_key` because f64 doesn't implement Ord.
            let location1 = self.location.haversine_distance_to(location1).meters();
            let location2 = self.location.haversine_distance_to(location2).meters();
            if let Some(ord) = location1.partial_cmp(&location2) {
                ord
            } else {
                Ordering::Equal
            }
        });

        provider_addresses
            .into_iter()
            .rev()
            .map(|(_, protocol, host, port)| format!("{protocol}://{host}:{port}"))
            .collect()
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

        let providers: Vec<&ProviderResult> = multihash_results
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

        let provider_addresses = self.provider_addresses(providers);

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
