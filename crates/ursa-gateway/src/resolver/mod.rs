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
use itertools::{Either, Itertools};
use libp2p::multiaddr::Protocol;
use maxminddb::{geoip2::City, Reader};
use model::IndexerResponse;
use moka::sync::Cache;
use ordered_float::OrderedFloat;
use serde_json::from_slice;
use std::sync::TryLockError;
use std::{collections::VecDeque, net::IpAddr, sync::Arc, sync::RwLock, time::Duration};
use tokio::time::Instant;
use tracing::{debug, error, warn};

use crate::{
    resolver::model::{Metadata, ProviderResult},
    util::error::Error,
};

const FLEEK_NETWORK_FILTER: &[u8] = b"FleekNetwork";
const MAX_DISTANCE: OrderedFloat<f64> = OrderedFloat(565_000f64);
const TIME_SHARING_DURATION: Duration = Duration::from_millis(500);

type Client = client::Client<HttpsConnector<HttpConnector>, Body>;

pub struct Resolver {
    indexer_cid_url: String,
    client: Client,
    cache: Cache<String, Addresses>,
    maxminddb: Arc<Reader<Vec<u8>>>,
    location: Location,
}

impl Resolver {
    pub fn new(
        indexer_cid_url: String,
        client: Client,
        cache: Cache<String, Addresses>,
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

    /// Returns a set of provider address sorted by their distance relative to the gateway.
    fn provider_addresses(&self, providers: Vec<&ProviderResult>) -> Addresses {
        let (neighbors, outsiders) = providers
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
                let distance = self.location.haversine_distance_to(&location).meters();
                if !distance.is_finite() {
                    debug!("Skipping {host} because distance could not be computed");
                    return None;
                }
                debug!("{host} is {distance:?} meters from host");
                Some((
                    OrderedFloat(distance),
                    format!("{protocol}://{host}:{port}"),
                ))
            })
            .partition_map(|(distance, address)| {
                if distance > MAX_DISTANCE {
                    debug!("Adding {address} to outsider list");
                    return Either::Right(address);
                }
                Either::Left(address)
            });
        Addresses {
            inner: Arc::new(RwLock::new(AddressesState {
                neighbors,
                outsiders,
                stamp: Instant::now(),
            })),
        }
    }

    pub async fn resolve_content(&self, cid: &str) -> Result<Response<Body>, Error> {
        let endpoint = format!("{}/{cid}", self.indexer_cid_url);

        let uri = endpoint.parse::<Uri>().map_err(|e| {
            error!("Error parsed uri: {endpoint} {e:?}");
            anyhow!("Error parsed uri: {endpoint}")
        })?;

        let provider_addresses = match self.cache.get(cid) {
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

                let providers: Vec<&ProviderResult> = indexer_response
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

                let provider_addresses = self.provider_addresses(providers);
                if provider_addresses.is_empty() {
                    return Err(Error::Internal(
                        "Failed to get a valid address for provider".to_string(),
                    ));
                }

                self.cache
                    .insert(cid.to_string(), provider_addresses.clone());

                provider_addresses
            }
            Some(provider_addresses) => provider_addresses,
        };

        debug!(
            "Provider addresses to query: {:?}",
            provider_addresses.neighbors()
        );

        while let Some(addr) = provider_addresses.next() {
            let endpoint = format!("{addr}/ursa/v0/{cid}");
            let uri = match endpoint.parse::<Uri>() {
                Ok(uri) => uri,
                Err(e) => {
                    error!("Error parsed uri: {endpoint} {e:?}");
                    continue;
                }
            };
            match self.client.get(uri).await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    provider_addresses.remove(addr);
                    error!("Error querying the node provider: {endpoint:?} {e:?}")
                }
            };
        }

        let outsiders = provider_addresses.outsiders();

        if !outsiders.is_empty() {
            debug!(
                "Failed to get content from neighbors so falling back to {:?}",
                outsiders
            );
        }

        for addr in outsiders {
            let endpoint = format!("{addr}/ursa/v0/{cid}");
            let uri = match endpoint.parse::<Uri>() {
                Ok(uri) => uri,
                Err(e) => {
                    error!("Error parsed uri: {endpoint} {e:?}");
                    continue;
                }
            };
            match self.client.get(uri).await {
                Ok(resp) => return Ok(resp),
                Err(e) => error!("Error querying the node provider: {endpoint:?} {e:?}"),
            };
        }

        // In the case that none of the addresses worked, we clean our cache.
        self.cache.invalidate(cid);
        Err(Error::Internal("Failed to get data".to_string()))
    }
}

#[derive(Clone)]
pub struct Addresses {
    inner: Arc<RwLock<AddressesState>>,
}

pub struct AddressesState {
    // Nodes within MAX_DISTANCE radius.
    neighbors: VecDeque<String>,
    // Nodes outside MAX_DISTANCE radius.
    outsiders: VecDeque<String>,
    // Recently viewed time stamp.
    stamp: Instant,
}

impl Addresses {
    fn next(&self) -> Option<String> {
        let inner = self.inner.read().unwrap();
        if inner.stamp + TIME_SHARING_DURATION >= Instant::now() {
            drop(inner);
            debug!("Time is up! Attempting to grab write lock to rotate");
            match self.inner.try_write() {
                Ok(mut writer_guard) => {
                    let front = writer_guard.neighbors.pop_front().unwrap();
                    writer_guard.neighbors.push_back(front.clone());
                    writer_guard.stamp = Instant::now();
                    Some(front)
                }
                Err(TryLockError::WouldBlock) => {
                    self.inner.read().unwrap().neighbors.front().cloned()
                }
                _ => panic!(""),
            }
        } else {
            inner.neighbors.front().cloned()
        }
    }

    fn neighbors(&self) -> VecDeque<String> {
        self.inner.read().unwrap().neighbors.clone()
    }

    fn outsiders(&self) -> VecDeque<String> {
        self.inner.read().unwrap().outsiders.clone()
    }

    fn remove(&self, addr: String) {
        let mut inner = self.inner.write().unwrap();
        inner.stamp = Instant::now();
        inner.neighbors.retain(|a| a != &addr);
    }

    fn is_empty(&self) -> bool {
        let inner = self.inner.read().unwrap();
        return inner.neighbors.is_empty() && inner.outsiders.is_empty();
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
