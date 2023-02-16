use anyhow::{anyhow, Result};
use geoutils::Location;
use libp2p::PeerId;
use maxminddb::geoip2::City;
use maxminddb::Reader;
use std::collections::HashSet;
use std::net::IpAddr;
use tracing::warn;

/// Manages a node's connected peers.
pub struct InnerManager {
    peers: HashSet<PeerId>,
    location: Location,
    maxminddb: Reader<Vec<u8>>,
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
            location,
            maxminddb,
        }))
    }

    pub fn insert(&mut self, peer: PeerId) -> bool {
        match self {
            Manager::PrivateNetwork(peers) => peers.insert(peer),
            Manager::PublicNetwork(manager) => manager.peers.insert(peer),
        }
    }

    pub fn contains(&self, peer: &PeerId) -> bool {
        match self {
            Manager::PrivateNetwork(peers) => peers.contains(peer),
            Manager::PublicNetwork(manager) => manager.peers.contains(peer),
        }
    }

    pub fn peers(&self) -> HashSet<PeerId> {
        match self {
            Manager::PrivateNetwork(peers) => peers.clone(),
            Manager::PublicNetwork(manager) => manager.peers.clone(),
        }
    }

    pub fn ref_peers(&self) -> &HashSet<PeerId> {
        match self {
            Manager::PrivateNetwork(peers) => &peers,
            Manager::PublicNetwork(manager) => &manager.peers,
        }
    }

    pub fn remove(&mut self, peer: &PeerId) -> bool {
        match self {
            Manager::PrivateNetwork(peers) => peers.remove(peer),
            Manager::PublicNetwork(manager) => manager.peers.remove(peer),
        }
    }
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
