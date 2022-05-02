//! Fnet Discovery implementation.
//!
//!
//!

use std::{
    collections::{HashMap, HashSet, VecDeque},
    task::{Context, Poll},
};

use libp2p::{
    core::{connection::ConnectionId, ConnectedPoint, PublicKey},
    kad::{handler::KademliaHandlerProto, store::MemoryStore, Kademlia, KademliaConfig, QueryId},
    swarm::{
        ConnectionHandler, IntoConnectionHandler, NetworkBehaviour, NetworkBehaviourAction,
        PollParameters,
    },
    Multiaddr, PeerId,
};

// use super::handler::DiscoveryEventHandler;

struct PeerInfo {
    peer_id: PeerId,
    addresses: Vec<Multiaddr>,
}

#[derive(Debug)]
pub enum DiscoveryEvent {}

pub struct DiscoveryBehaviour {
    local_peer_id: PeerId,
    /// should we support MDNS?
    /// kad instance
    kademlia: Kademlia<MemoryStore>,
    /// boostrap nodes
    /// could merge the bootstrap nodes under [peers]
    boostrap: Vec<(PeerId, Multiaddr)>,
    /// connected peers
    peers: HashSet<PeerId>,
    /// information about connected peers
    /// we should prob introduce and arc lock on this
    peer_info: HashMap<PeerId, PeerInfo>,
    /// events
    events: VecDeque<DiscoveryEvent>,
    // Custom event handler
    // events: VecDeque<NetworkBehaviourAction<DiscoveryEvent, DiscoveryEventHandler>>,
}

impl DiscoveryBehaviour {
    /**
        Abstract the bootstrapping nodes in [FnetConfig]
    */
    // pub fn new(local_public_key: PublicKey, boostrap: Vec<(PeerId, Multiaddr)>) {
    pub fn new(local_public_key: PublicKey) -> Self {
        // setup kademlia config
        // move to FnetConfig
        let kademlia = {
            let name = "";
            let replication_factor = "";
            let peer_id = local_public_key.to_peer_id();
            let store = MemoryStore::new(peer_id);

            let config = KademliaConfig::default()
                .set_protocol_name(name)
                .set_replication_factor(replication_factor);

            Kademlia::with_config(peer_id, store, config)
        };

        // boostrap kademlia

        // future: relay circuit v2 / hole punching
    }

    pub fn peer_info(&self) -> HashMap<PeerId, PeerInfo> {
        &self.peer_info()
    }
}

impl NetworkBehaviour for DiscoveryBehaviour {
    /// Custom handler todo
    // type ConnectionHandler = DiscoveryHandler;
    type ConnectionHandler = KademliaHandlerProto<QueryId>;

    type OutEvent = DiscoveryEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        self.kademlia.new_handler()
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        self.peer_info
            .get(peer_id)
            .map(|peer_info| peer_info.addresses.cloned().collect())
            .unwrap_or_default()
    }

    fn inject_connection_established(
        &mut self,
        peer_id: &PeerId,
        connection_id: &ConnectionId,
        endpoint: &ConnectedPoint,
        failed_addresses: Option<&Vec<Multiaddr>>,
        other_established: usize,
    ) {
        self.kademlia.inject_connection_established(
            peer_id,
            connection_id,
            endpoint,
            failed_addresses,
            other_established,
        );
    }

    fn inject_connection_closed(
        &mut self,
        peer_id: &PeerId,
        connection_id: &ConnectionId,
        endpoint: &ConnectedPoint,
        handler: <Self::ConnectionHandler as IntoConnectionHandler>::Handler,
        remaining_established: usize,
    ) {
        self.kademlia.inject_connection_closed(
            peer_id,
            connection_id,
            endpoint,
            handler,
            remaining_established,
        );
    }

    fn inject_event(
        &mut self,
        peer_id: PeerId,
        connection: ConnectionId,
        event: <<Self::ConnectionHandler as IntoConnectionHandler>::Handler as ConnectionHandler>::OutEvent,
    ) {
        self.kademlia.inject_event(peer_id, connection, event);
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        params: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Self::OutEvent, Self::ConnectionHandler>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
