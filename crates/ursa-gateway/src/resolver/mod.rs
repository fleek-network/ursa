pub mod model;
mod round_robin;

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
use std::{net::IpAddr, sync::Arc};
use tracing::{debug, error, warn};

use crate::{
    resolver::{
        model::{Metadata, ProviderResult},
        round_robin::Queue,
    },
    util::error::Error,
};

const FLEEK_NETWORK_FILTER: &[u8] = b"FleekNetwork";
const MAX_DISTANCE: OrderedFloat<f64> = OrderedFloat(565_000f64);

type Client = client::Client<HttpsConnector<HttpConnector>, Body>;

pub struct Resolver {
    indexer_cid_url: String,
    client: Client,
    cache: Cache<String, Arc<Providers>>,
    maxminddb: Arc<Reader<Vec<u8>>>,
    location: GatewayLocation,
}

impl Resolver {
    pub fn new(
        indexer_cid_url: String,
        client: Client,
        cache: Cache<String, Arc<Providers>>,
        maxminddb: Arc<Reader<Vec<u8>>>,
        addr: IpAddr,
    ) -> Result<Self, Error> {
        let location = if addr.is_loopback() {
            GatewayLocation::Private
        } else {
            let city = maxminddb
                .lookup::<City>(addr)
                .map_err(|e| anyhow!(e.to_string()))?;
            GatewayLocation::Public(get_location(city)?)
        };

        Ok(Self {
            indexer_cid_url,
            client,
            cache,
            maxminddb,
            location,
        })
    }

    /// Partitions the providers into a set containing providers that are within
    /// MAX_DISTANCE distance from the gateway and another set of providers that
    /// are outside of that distance.
    fn partition_providers(&self, providers: Vec<&ProviderResult>) -> Providers {
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
                let distance = match self.location {
                    GatewayLocation::Private => 0f64,
                    GatewayLocation::Public(gateway_location) => {
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
                        let provider_location = get_location(city?)
                            .map_err(|e| {
                                debug!("Failed to get location for city with ip {host} {:?}", e)
                            })
                            .ok()?;
                        let distance = gateway_location
                            .haversine_distance_to(&provider_location)
                            .meters();
                        if !distance.is_finite() {
                            debug!("Skipping {host} because distance could not be computed");
                            return None;
                        }
                        distance
                    }
                };
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
        Providers {
            neighbors: Queue::new(neighbors),
            outsiders: Queue::new(outsiders),
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

                let providers = self.partition_providers(providers);
                if providers.neighbors.is_empty() && providers.outsiders.is_empty() {
                    return Err(Error::Internal(
                        "Failed to get a valid address for provider".to_string(),
                    ));
                }
                let providers = Arc::new(providers);
                self.cache.insert(cid.to_string(), providers.clone());

                providers
            }
            Some(providers) => providers,
        };

        debug!(
            "Provider addresses to query: {:?}",
            provider_addresses.neighbors
        );

        while let Some(addr) = provider_addresses.neighbors.next() {
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
                    provider_addresses.neighbors.remove(addr);
                    error!("Error querying the node provider: {endpoint:?} {e:?}")
                }
            };
        }

        if !provider_addresses.outsiders.is_empty() {
            debug!(
                "Failed to get content from neighbors so falling back to {:?}",
                provider_addresses.outsiders
            );
        }

        while let Some(addr) = provider_addresses.outsiders.next() {
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
                    provider_addresses.outsiders.remove(addr);
                    error!("Error querying the node provider: {endpoint:?} {e:?}")
                }
            };
        }

        // In the case that none of the addresses worked, we clean our cache.
        self.cache.invalidate(cid);
        Err(Error::Internal("Failed to get data".to_string()))
    }
}

pub struct Providers {
    neighbors: Queue<String>,
    outsiders: Queue<String>,
}

pub enum GatewayLocation {
    Private,
    Public(Location),
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
