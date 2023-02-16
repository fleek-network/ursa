use libp2p::PeerId;
use std::collections::HashSet;

/// Manages a node's connected peers.
pub struct Manager {
    peers: HashSet<PeerId>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            peers: HashSet::new(),
        }
    }

    pub fn insert(&mut self, peer: PeerId) -> bool {
        self.peers.insert(peer)
    }

    pub fn contains(&self, peer: &PeerId) -> bool {
        self.peers.contains(peer)
    }

    pub fn peers(&self) -> HashSet<PeerId> {
        self.peers.clone()
    }

    pub fn ref_peers(&self) -> &HashSet<PeerId> {
        &self.peers
    }

    pub fn remove(&mut self, peer: &PeerId) -> bool {
        self.peers.remove(peer)
    }
}
