use libp2p::{
    kad::{handler::KademliaHandlerProto, QueryId},
    swarm::{
        ConnectionHandler, IntoConnectionHandler, NetworkBehaviour, NetworkBehaviourAction,
        PollParameters,
    },
    PeerId,
};

#[derive(Debug)]
pub enum DiscoveryEvent {}

pub struct DiscoveryBehaviour {}

impl NetworkBehaviour for DiscoveryBehaviour {
    type ConnectionHandler = KademliaHandlerProto<QueryId>;

    type OutEvent = DiscoveryEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        todo!()
    }

    fn inject_event(
        &mut self,
        peer_id: PeerId,
        connection: libp2p::core::connection::ConnectionId,
        event: <<Self::ConnectionHandler as IntoConnectionHandler>::Handler as ConnectionHandler>::OutEvent,
    ) {
        todo!()
    }

    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
        params: &mut impl PollParameters,
    ) -> std::task::Poll<NetworkBehaviourAction<Self::OutEvent, Self::ConnectionHandler>> {
        todo!()
    }
}
