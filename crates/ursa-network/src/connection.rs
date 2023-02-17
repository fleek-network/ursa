use anyhow::{anyhow, Result};
use geoutils::Location;
use libp2p::multiaddr::Protocol;
use libp2p::{Multiaddr, PeerId};
use maxminddb::geoip2::City;
use maxminddb::Reader;
use ordered_float::OrderedFloat;
use std::cmp::Ordering;
use std::collections::hash_set::Iter;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::net::IpAddr;
use tracing::warn;

#[derive(Clone)]
pub struct Connection {
    peer: PeerId,
    distance: OrderedFloat<f64>,
}

impl PartialEq<Self> for Connection {
    fn eq(&self, other: &Self) -> bool {
        self.distance.eq(&other.distance)
    }
}

impl Eq for Connection {}

impl PartialOrd<Self> for Connection {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.distance.partial_cmp(&other.distance)
    }
}

impl Ord for Connection {
    fn cmp(&self, other: &Self) -> Ordering {
        self.distance.cmp(&other.distance)
    }
}

/// Manages a node's connected peers.
pub struct InnerManager {
    peers: HashMap<PeerId, Option<Connection>>,
    close_peers: Vec<Connection>,
    location: Location,
    maxminddb: Reader<Vec<u8>>,
}

impl InnerManager {
    fn get_distance(&self, addr: IpAddr) -> Option<f64> {
        let city = self.maxminddb.lookup::<City>(addr).ok()?;
        let location = get_location(city).ok()?;
        let distance = self.location.haversine_distance_to(&location);
        Some(distance.meters())
    }
}

pub enum Manager {
    PrivateNetwork(HashSet<PeerId>),
    PublicNetwork(InnerManager),
}

impl Manager {
    pub fn new_private_network_manager() -> Self {
        Self::PrivateNetwork(HashSet::new())
    }

    pub fn new_public_network_manager(addr: IpAddr, maxminddb: Reader<Vec<u8>>) -> Result<Self> {
        let city = maxminddb.lookup::<City>(addr)?;
        let location = get_location(city)?;

        Ok(Self::PublicNetwork(InnerManager {
            peers: HashMap::new(),
            close_peers: Vec::new(),
            location,
            maxminddb,
        }))
    }

    pub fn insert(&mut self, peer: PeerId, addr: Multiaddr) -> bool {
        match self {
            Self::PrivateNetwork(peers) => peers.insert(peer),
            Self::PublicNetwork(manager) => {
                let connection =
                    get_ip(addr)
                        .and_then(|ip| manager.get_distance(ip))
                        .map(|distance| Connection {
                            peer,
                            distance: OrderedFloat(distance),
                        });
                if let Some(connection) = connection.clone() {
                    manager.close_peers.push(connection);
                    manager.close_peers.sort();
                }
                manager.peers.insert(peer, connection).is_none()
            }
        }
    }

    pub fn contains(&self, peer: &PeerId) -> bool {
        match self {
            Self::PrivateNetwork(peers) => peers.contains(peer),
            Self::PublicNetwork(manager) => manager.peers.contains_key(peer),
        }
    }

    pub fn peers(&self) -> Vec<PeerId> {
        match self {
            Self::PrivateNetwork(peers) => peers.clone().into_iter().collect(),
            Self::PublicNetwork(manager) => manager.peers.clone().into_keys().collect(),
        }
    }

    pub fn remove(&mut self, peer: &PeerId) -> bool {
        match self {
            Self::PrivateNetwork(peers) => peers.remove(peer),
            Self::PublicNetwork(manager) => {
                // TODO: replace by another.
                manager.close_peers = manager
                    .close_peers
                    .clone()
                    .into_iter()
                    .filter(|c| c.peer != *peer)
                    .collect();
                manager.close_peers.sort();
                manager.peers.remove(peer).is_some()
            }
        }
    }
}

fn get_ip(multiaddr: Multiaddr) -> Option<IpAddr> {
    for comp in multiaddr.iter() {
        match comp {
            Protocol::Ip4(ip) => return Some(ip.into()),
            Protocol::Ip6(ip) => return Some(ip.into()),
            _ => {}
        }
    }
    None
}

fn get_location(city: City) -> Result<Location> {
    let location = city.location.ok_or_else(|| anyhow!("missing location"))?;
    let latitude = location
        .latitude
        .ok_or_else(|| anyhow!("missing latitude"))?;
    let longitude = location
        .longitude
        .ok_or_else(|| anyhow!("missing longitude"))?;
    Ok(Location::new(latitude, longitude))
}
