use anyhow::{anyhow, Result};
use geoutils::Location;
use libp2p::multiaddr::Protocol;
use libp2p::{Multiaddr, PeerId};
use maxminddb::geoip2::City;
use maxminddb::Reader;
use ordered_float::OrderedFloat;
use std::collections::HashSet;
use std::net::IpAddr;

const MAX_DISTANCE: OrderedFloat<f64> = OrderedFloat(10000f64);
const NEIGHBORHOOD_SIZE: usize = 3;

/// Manages a node's connected peers.
pub struct InnerManager {
    peers: HashSet<PeerId>,
    closest_peers: HashSet<PeerId>,
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
            peers: HashSet::new(),
            closest_peers: HashSet::new(),
            location,
            maxminddb,
        }))
    }

    pub fn insert(&mut self, peer: PeerId, addr: Multiaddr) -> bool {
        match self {
            Self::PrivateNetwork(peers) => peers.insert(peer),
            Self::PublicNetwork(manager) => {
                let distance = get_ip(addr)
                    .map(|ip| manager.get_distance(ip))
                    .flatten()
                    .map(OrderedFloat);
                if let Some(distance) = distance {
                    if manager.closest_peers.len() < NEIGHBORHOOD_SIZE
                        && distance.is_finite()
                        && distance < MAX_DISTANCE
                    {
                        manager.closest_peers.insert(peer);
                    }
                }
                manager.peers.insert(peer)
            }
        }
    }

    pub fn contains(&self, peer: &PeerId) -> bool {
        match self {
            Self::PrivateNetwork(peers) => peers.contains(peer),
            Self::PublicNetwork(manager) => manager.peers.contains(peer),
        }
    }

    pub fn ref_peers(&self) -> impl Iterator<Item = &PeerId> + '_ {
        match self {
            Manager::PrivateNetwork(peers) => peers.iter(),
            Manager::PublicNetwork(manager) => manager.peers.iter(),
        }
    }

    pub fn peers(&self) -> Vec<PeerId> {
        match self {
            Self::PrivateNetwork(peers) => peers.clone().into_iter().collect(),
            Self::PublicNetwork(manager) => manager.peers.clone().into_iter().collect(),
        }
    }

    pub fn remove(&mut self, peer: &PeerId) -> bool {
        match self {
            Self::PrivateNetwork(peers) => peers.remove(peer),
            Self::PublicNetwork(manager) => {
                manager.closest_peers.remove(peer);
                manager.peers.remove(peer)
            }
        }
    }

    pub fn closest_peers(&self) -> Vec<PeerId> {
        if let Self::PublicNetwork(manager) = self {
            manager.closest_peers.clone().into_iter().collect()
        } else {
            self.peers()
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
