use libp2p::PeerId;
use std::collections::HashMap;
use std::{collections::HashSet, time::Duration};
use tracing::debug;

const MESH_MAX_SIZE: usize = 3;
const MAX_RTT: Duration = Duration::from_millis(15);

pub struct Manager {
    /// Connected peers.
    connected_peers: HashSet<PeerId>,
    /// Set of peers to use in content replication.
    replication_set: HashMap<PeerId, Duration>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            connected_peers: HashSet::new(),
            replication_set: HashMap::new(),
        }
    }

    pub fn handle_rtt_received(&mut self, rtt: Duration, peer: PeerId) {
        if rtt > MAX_RTT {
            self.replication_set.remove(&peer);
            debug!("{peer} was not added because of high rrt {rtt:?}");
            return;
        }
        if !self.connected_peers.contains(&peer) {
            debug!("{peer} was not added because we don't have a connection for it");
            return;
        }
        if self.replication_set.len() >= MESH_MAX_SIZE && !self.replication_set.contains_key(&peer)
        {
            if let Some(peer_with_max_rtt) = self
                .replication_set
                .iter()
                .max_by_key(|(_, duration)| duration.clone())
                .filter(|(_, duration)| duration > &&rtt)
                .map(|(peer, _)| peer.clone())
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
}
