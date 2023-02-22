pub mod model;

use anyhow::{anyhow, Context};
use axum::response::IntoResponse;
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
use iroh::get::Options;
use iroh::protocol::AuthToken;
use iroh::{Hash, PeerId};
use libp2p::multiaddr::Protocol;
use maxminddb::{geoip2::City, Reader};
use model::IndexerResponse;
use moka::sync::Cache;
use ordered_float::OrderedFloat;
use serde_json::from_slice;
use std::net::SocketAddr;
use std::str::FromStr;
use std::{net::IpAddr, sync::Arc};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, warn};

use crate::{
    resolver::model::{Metadata, ProviderResult},
    util::error::Error,
};

const FLEEK_NETWORK_FILTER: &[u8] = b"FleekNetwork";

type Client = client::Client<HttpsConnector<HttpConnector>, Body>;

pub struct Resolver {
    indexer_cid_url: String,
    client: Client,
    cache: Cache<String, Vec<String>>,
    maxminddb: Arc<Reader<Vec<u8>>>,
    location: Location,
}

impl Resolver {
    pub fn new(
        indexer_cid_url: String,
        client: Client,
        cache: Cache<String, Vec<String>>,
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
                let distance = self.location.haversine_distance_to(&location).meters();
                Some((distance, protocol, host, port.unwrap()))
            })
            .filter_map(|(distance, protocol, host, port)| {
                if distance.is_finite() {
                    debug!("{host} is {distance:?} meters from host");
                    Some((OrderedFloat(distance), protocol, host, port))
                } else {
                    debug!("Skipping {host} because distance could not be computed");
                    None
                }
            })
            .collect::<Vec<(OrderedFloat<f64>, String, IpAddr, u16)>>();

        provider_addresses.sort_by(|(totally_ordered1, _, _, _), (totally_ordered2, _, _, _)| {
            totally_ordered1.cmp(totally_ordered2)
        });

        provider_addresses
            .into_iter()
            .map(|(_, protocol, host, port)| format!("{protocol}://{host}:{port}"))
            .collect()
    }

    pub async fn resolve_content(&self, cid: &str) -> Result<Response<Body>, Error> {
        let endpoint = format!("{}/{cid}", self.indexer_cid_url);

        let uri = endpoint.parse::<Uri>().map_err(|e| {
            error!("Error parsed uri: {endpoint} {e:?}");
            anyhow!("Error parsed uri: {endpoint}")
        })?;

        let _ = match self.cache.get(cid) {
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

                debug!("Provider addresses to query: {provider_addresses:?}");

                self.cache
                    .insert(cid.to_string(), provider_addresses.clone());

                provider_addresses
            }
            Some(provider_addresses) => provider_addresses,
        };

        // For testing.
        let ADDR = "0.0.0.0".to_string();
        let USE_IROH: bool = true;
        let HASH: Hash = Hash::from([
            127, 238, 212, 156, 13, 112, 45, 206, 75, 86, 19, 77, 213, 193, 170, 157, 58, 207, 17,
            220, 242, 142, 196, 36, 238, 77, 3, 46, 26, 162, 183, 138,
        ]);
        let AUTH_TOKEN: AuthToken =
            AuthToken::from_str("4_4v9BF7BK87fJkDkO_MO4prTNdg0zTtpRMR88CyrQE")
                .context("Auth token failed")?;
        let PEER_ID: PeerId = PeerId::from_str("ybRTLgbj9iF4TxWujNGt-B4hVAIrBTZjLXVXvgJ-Ywo")
            .context("Peer id parsing failed")?;
        let mut opts = Options {
            peer_id: Some(PEER_ID),
            // addr: SocketAddr::from_str(&format!("{ADDR}:4433")).unwrap(),
            ..Default::default()
        };
        let on_connected = || async { Ok(()) };
        let on_collection = |collection: &iroh::blobs::Collection| async { Ok(()) };
        let (tx, mut rx) = mpsc::channel(1);
        let on_blob = |hash: Hash, mut reader, name: String| {
            let txx = tx.clone();
            async move {
                let mut buf = vec![];
                tokio::io::copy(&mut reader, &mut buf).await?;
                txx.send(buf).await.expect("Sending to succeed");
                Ok(reader)
            }
        };

        if USE_IROH {
            if let Err(e) =
                iroh::get::run(HASH, AUTH_TOKEN, opts, on_connected, on_collection, on_blob).await
            {
                error!("There was an error {e}");
                return Err(Error::Internal("Failed to get data".to_string()));
            }
            let data = rx.recv().await.expect("Receive to succeed");
            return Ok(Response::new(Body::from(data)));
        } else {
            let endpoint = format!("http://{ADDR}/ursa/v0/{cid}");
            match endpoint.parse::<Uri>() {
                Ok(uri) => {
                    match self.client.get(uri).await {
                        Ok(resp) => {
                            return Ok(resp);
                        }
                        Err(e) => error!("Error querying the node provider: {endpoint:?} {e:?}"),
                    };
                }
                Err(e) => {
                    error!("Error parsed uri: {endpoint} {e:?}");
                }
            }
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
