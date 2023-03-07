use libp2p::PeerId;
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use tracing::debug;

#[cfg(not(test))]
const REPLICATION_MAX_SIZE: usize = 2;
#[cfg(not(test))]
const MAX_RTT: Duration = Duration::from_millis(15);

#[derive(Default)]
pub struct Manager {
    /// Connected peers.
    connected_peers: HashSet<PeerId>,
    /// Set of peers to use in content replication.
    replication_set: HashMap<PeerId, Duration>,
}

impl Manager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, peer: PeerId) -> bool {
        self.connected_peers.insert(peer)
    }

    pub fn contains(&self, peer: &PeerId) -> bool {
        self.connected_peers.contains(peer)
    }

    pub fn ref_peers(&self) -> &HashSet<PeerId> {
        &self.connected_peers
    }

    pub fn peers(&self) -> HashSet<PeerId> {
        self.connected_peers.clone()
    }

    pub fn remove(&mut self, peer: &PeerId) -> bool {
        self.replication_set.remove(peer);
        self.connected_peers.remove(peer)
    }

    pub fn replication_set(&self) -> Vec<PeerId> {
        self.replication_set.clone().into_keys().collect()
    }

    #[cfg(not(test))]
    pub fn handle_rtt_received(&mut self, rtt: Duration, peer: PeerId) {
        debug!("Received {rtt:?} rtt for {peer}");
        if rtt > MAX_RTT {
            self.replication_set.remove(&peer);
            debug!("{peer} was not added because of high rrt");
            return;
        }
        if !self.connected_peers.contains(&peer) {
            debug!("{peer} was not added because we don't have a connection for it");
            return;
        }
        if self.replication_set.len() >= REPLICATION_MAX_SIZE
            && !self.replication_set.contains_key(&peer)
        {
            if let Some(peer_with_max_rtt) = self
                .replication_set
                .iter()
                .max_by_key(|(_, duration)| **duration)
                .filter(|(_, duration)| duration > &&rtt)
                .map(|(peer, _)| *peer)
            {
                debug!("Removing {} from mesh", peer_with_max_rtt);
                self.replication_set.remove(&peer_with_max_rtt);
                debug!("Adding {peer} to mesh");
                self.replication_set.insert(peer, rtt);
            }
        } else {
            debug!("Adding/updating mesh with {peer}");
            self.replication_set.insert(peer, rtt);
        }
    }

    #[cfg(test)]
    pub fn handle_rtt_received(&mut self, rtt: Duration, peer: PeerId) {
        debug!("Ignoring rtt and inserting {peer}");
        self.replication_set.insert(peer, rtt);
    }
}
