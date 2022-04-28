//! Fnet Discovery implementation.
//!
//!
//!

use std::{
    collections::{HashMap, HashSet, VecDeque},
    task::{Context, Poll},
};

use libp2p::{
    core::{connection::ConnectionId, ConnectedPoint},
    kad::{handler::KademliaHandlerProto, store::MemoryStore, Kademlia, KademliaEvent, QueryId},
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
    /// kad instance
    kademlia: Kademlia<MemoryStore>,
    /// boostrap nodes
    /// could merge the bootstrap nodes under [peers]
    boostrap: HashSet<PeerId>,
    /// connected peers
    peers: HashSet<PeerId>,
    /// information about connected peers
    peer_info: HashMap<PeerId, PeerInfo>,
    /// events
    events: VecDeque<DiscoveryEvent>,
    // Custom event handler
    // events: VecDeque<NetworkBehaviourAction<DiscoveryEvent, DiscoveryEventHandler>>,
}

impl NetworkBehaviour for DiscoveryBehaviour {
    type ConnectionHandler = KademliaHandlerProto<QueryId>;

    type OutEvent = DiscoveryEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        self.kademlia.new_handler()
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        todo!()
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
