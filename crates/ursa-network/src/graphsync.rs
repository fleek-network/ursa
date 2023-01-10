use graphsync::{HandlerProto, Request};
use ipld_traversal::blockstore::Blockstore;
use libp2p::core::connection::ConnectionId;
use libp2p::core::transport::ListenerId;
use libp2p::core::ConnectedPoint;
use libp2p::swarm::{
    ConnectionHandler, DialError, IntoConnectionHandler, NetworkBehaviour, NetworkBehaviourAction,
    PollParameters,
};
use libp2p::{Multiaddr, PeerId};
use std::task::{Context, Poll};

pub struct GraphSync<S>
where
    S: Blockstore + Send + Clone + 'static,
{
    graphsync: graphsync::GraphSync<S>,
    _tasks: Vec<Request>,
    _inflight: Vec<Request>,
}

impl<S> GraphSync<S>
where
    S: Blockstore + Send + Clone + 'static,
{
    pub fn new(store: S) -> Self {
        Self {
            graphsync: graphsync::GraphSync::new(store),
            _tasks: Vec::new(),
            _inflight: Vec::new(),
        }
    }

    pub fn add_address(&mut self, peer: &PeerId, addr: Multiaddr) {
        self.graphsync.add_address(peer, addr)
    }

    pub fn request(&mut self, peer: PeerId, request: Request) {
        self.graphsync.request(peer, request)
    }
}

impl<S> NetworkBehaviour for GraphSync<S>
where
    S: Blockstore + Send + Clone + 'static,
{
    type ConnectionHandler = <graphsync::GraphSync<S> as NetworkBehaviour>::ConnectionHandler;
    type OutEvent = <graphsync::GraphSync<S> as NetworkBehaviour>::OutEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        HandlerProto
    }

    fn addresses_of_peer(&mut self, peer: &PeerId) -> Vec<Multiaddr> {
        self.graphsync.addresses_of_peer(peer)
    }

    #[allow(deprecated)]
    fn inject_connection_established(
        &mut self,
        peer_id: &PeerId,
        conn: &ConnectionId,
        endpoint: &ConnectedPoint,
        failed_addresses: Option<&Vec<Multiaddr>>,
        other_established: usize,
    ) {
        self.graphsync.inject_connection_established(
            peer_id,
            conn,
            endpoint,
            failed_addresses,
            other_established,
        )
    }

    #[allow(deprecated)]
    fn inject_connection_closed(
        &mut self,
        peer_id: &PeerId,
        conn: &ConnectionId,
        point: &ConnectedPoint,
        handler: <Self::ConnectionHandler as IntoConnectionHandler>::Handler,
        remaining_established: usize,
    ) {
        self.graphsync.inject_connection_closed(
            peer_id,
            conn,
            point,
            handler,
            remaining_established,
        )
    }

    #[allow(deprecated)]
    fn inject_event(
        &mut self,
        peer_id: PeerId,
        connection: ConnectionId,
        event: <<Self::ConnectionHandler as IntoConnectionHandler>::Handler as ConnectionHandler>::OutEvent,
    ) {
        self.graphsync.inject_event(peer_id, connection, event)
    }

    fn inject_dial_failure(
        &mut self,
        _peer_id: Option<PeerId>,
        _: Self::ConnectionHandler,
        _error: &DialError,
    ) {
    }

    fn inject_new_listen_addr(&mut self, _id: ListenerId, _addr: &Multiaddr) {}

    fn inject_expired_listen_addr(&mut self, _id: ListenerId, _addr: &Multiaddr) {}

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        params: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Self::OutEvent, Self::ConnectionHandler>> {
        self.graphsync.poll(cx, params)
    }
}
