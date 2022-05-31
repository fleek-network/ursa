//! Ursa Discovery implementation.
//!
//!
//!

use std::{
    collections::{HashMap, HashSet, VecDeque},
    task::{Context, Poll},
};

use anyhow::{anyhow, Result};
use libp2p::{
    autonat::{Behaviour as Autonat, Config as AutonatConfig},
    core::{connection::ConnectionId, ConnectedPoint},
    kad::{
        handler::KademliaHandlerProto, store::MemoryStore, Kademlia, KademliaConfig, KademliaEvent,
        QueryId, QueryResult,
    },
    mdns::{Mdns, MdnsConfig},
    relay::v2::relay::{Config as RelayConfig, Relay},
    swarm::{
        behaviour::toggle::Toggle, ConnectionHandler, IntoConnectionHandler, NetworkBehaviour,
        NetworkBehaviourAction, PollParameters,
    },
    Multiaddr, PeerId,
};

use crate::config::UrsaConfig;
// use super::handler::DiscoveryEventHandler;

struct PeerInfo {
    peer_id: PeerId,
    addresses: Vec<Multiaddr>,
}

#[derive(Debug)]
pub enum DiscoveryEvent {
    Discoverd(PeerId),
    UnroutablePeer(PeerId),
}

pub struct DiscoveryBehaviour {
    local_peer_id: PeerId,
    /// Kademlia instance.
    kademlia: Kademlia<MemoryStore>,
    /// Boostrap nodes.
    bootstrap_nodes: Vec<(PeerId, Multiaddr)>,
    /// Connected peers.
    peers: HashSet<PeerId>,
    /// Information about connected peers.
    peer_info: HashMap<PeerId, PeerInfo>,
    /// events
    events: VecDeque<DiscoveryEvent>,
    /// Relay v2 for routing through peers.
    relay: Relay,
    /// Optional MDNS protocol.
    mdns: Toggle<Mdns>,
    /// Optional autonat.
    autonat: Toggle<Autonat>,
    // Custom event handler
    // events: VecDeque<NetworkBehaviourAction<DiscoveryEvent, DiscoveryEventHandler>>
}

impl DiscoveryBehaviour {
    pub fn new(config: &UrsaConfig) -> Result<Self> {
        let local_peer_id = PeerId::from(config.keypair.public());

        // setup kademlia config
        let kademlia = {
            let name = "";
            let replication_factor = "";
            let store = MemoryStore::new(local_peer_id);

            let config = KademliaConfig::default()
                .set_protocol_name(name)
                .set_replication_factor(replication_factor);
            // what more do we need to setup with Kad?

            Kademlia::with_config(local_peer_id, store, config)
        };

        // mdns is off by default
        let mdns = if config.mdns {
            Some(Mdns::new(MdnsConfig::default())).expect("mdns start")
        } else {
            None
        };

        // autonat is off by default
        let autonat = if config.autonat {
            let mut behaviour = Autonat::new(local_peer_id, AutonatConfig::default());

            for (peer, address) in config.bootstrap_nodes {
                behaviour.add_server(peer, Some(address));
            }

            behaviour
        } else {
            None
        };

        let relay = Relay::new(local_peer_id, RelayConfig::default());

        Ok(Self {
            local_peer_id,
            kademlia,
            bootstrap_nodes: Vec::new(),
            peers: HashSet::new(),
            peer_info: HashMap::new(),
            events: VecDeque::new(),
            relay,
            mdns: mdns.into(),
            autonat: autonat.into(),
        })
    }

    pub fn add_address(&self, peer_id: PeerId, address: Multiaddr) {
        &self.kademlia.add_address(&peer_id, address);
    }

    pub fn peers(&self) -> HashSet<PeerId> {
        &self.peers
    }

    pub fn peer_info(&self) -> HashMap<PeerId, PeerInfo> {
        &self.peer_info
    }

    pub fn boostrap(&self) -> Result<QueryId, String> {
        for (peer_id, address) in &self.bootstrap_nodes {
            &self.kademlia.add_address(peer_id, address.clone());
        }

        &self
            .kademlia
            .bootstrap()
            .map_err(|err| anyhow!("{:?}", err))
    }

    pub fn with_bootstrap_nodes(&mut self, bootstrap_nodes: Vec<(PeerId, Multiaddr)>) -> &mut Self {
        self.bootstrap_nodes.extend(bootstrap_nodes);
        self
    }

    fn handle_event(&self, event: KademliaEvent) {
        match event {
            KademliaEvent::OutboundQueryCompleted { result, .. } => match result {
                QueryResult::GetClosestPeers(result) => match result {
                    Ok(closet_peers_result) => {
                        let peers = closet_peers_result.peers;

                        todo!()
                    }
                    Err(_) => todo!(),
                },
                QueryResult::Bootstrap { .. }
                | QueryResult::GetRecord { .. }
                | QueryResult::PutRecord { .. }
                | QueryResult::GetProviders { .. }
                | QueryResult::StartProviding { .. }
                | QueryResult::RepublishProvider { .. }
                | QueryResult::RepublishRecord { .. } => {}
            },
            KademliaEvent::RoutingUpdated { .. }
            | KademliaEvent::RoutablePeer { .. }
            | KademliaEvent::InboundRequest { .. }
            | KademliaEvent::UnroutablePeer { .. }
            | KademliaEvent::PendingRoutablePeer { .. } => {}
        }
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
        match self.events.pop_front() {
            Some(event) => Poll::Ready(NetworkBehaviourAction::GenerateEvent(event)),
            None => todo!(),
            _ => Poll::Pending,
        }

        // Poll kademlia for events
        while let Poll::Ready(action) = self.kademlia.poll(cx, params) {
            match action {
                NetworkBehaviourAction::GenerateEvent(event) => self.handle_event(event),
                NetworkBehaviourAction::Dial { opts, handler } => {
                    Poll::Ready(NetworkBehaviourAction::Dial { opts, handler })
                }
                NetworkBehaviourAction::NotifyHandler {
                    peer_id,
                    handler,
                    event,
                } => Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                    peer_id,
                    handler,
                    event,
                }),
                NetworkBehaviourAction::ReportObservedAddr { address, score } => {
                    Poll::Ready(NetworkBehaviourAction::ReportObservedAddr { address, score })
                }
                NetworkBehaviourAction::CloseConnection {
                    peer_id,
                    connection,
                } => Poll::Ready(NetworkBehaviourAction::CloseConnection {
                    peer_id,
                    connection,
                }),
            }
        }
    }
}
