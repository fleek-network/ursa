use libp2p::PeerId;
use std::collections::HashMap;
use std::{collections::HashSet, time::Duration};
use tracing::debug;

const MESH_MAX_SIZE: usize = 3;
const MAX_RTT: Duration = Duration::from_millis(15);

pub struct Manager {
    connected_peers: HashSet<PeerId>,
    mesh_peers: HashMap<PeerId, Duration>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            connected_peers: HashSet::new(),
            mesh_peers: HashMap::new(),
        }
    }

    pub fn handle_rtt_received(&mut self, rtt: Duration, peer: PeerId) {
        if rtt > MAX_RTT {
            debug!("{peer} was not added because of high rrt {rtt:?}");
            return;
        }
        if !self.connected_peers.contains(&peer) {
            debug!("{peer} was not added because we don't have a connection for it");
            return;
        }
        if self.mesh_peers.len() > MESH_MAX_SIZE {
            if let Some(peer_with_max_rtt) = self
                .mesh_peers
                .iter()
                .max_by_key(|(_, duration)| duration)
                .map(|(peer, _)| peer)
            {
                debug!("Removing {} from mesh", peer_with_max_rtt);
                self.mesh_peers.remove(peer_with_max_rtt);
                debug!("Adding {peer} to mesh");
                self.mesh_peers.insert(peer, rtt);
            }
        } else {
            debug!("Adding {peer} to mesh");
            self.mesh_peers.insert(peer, rtt);
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
        self.mesh_peers.remove(peer);
        self.connected_peers.remove(peer)
    }

    pub fn mesh_peers(&self) -> Vec<PeerId> {
        self.mesh_peers
            .iter()
            .map(|(peer, _)| peer.clone())
            .collect()
    }
}
