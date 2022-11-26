//! Ursa Discovery implementation.
//!
//!
//!

use std::{
    collections::{HashMap, HashSet, VecDeque},
    num::NonZeroUsize,
    task::{Context, Poll},
};

use crate::config::UrsaConfig;
use anyhow::{anyhow, Error, Result};
use futures::executor::block_on;
use libp2p::{
    core::{connection::ConnectionId, ConnectedPoint},
    identity::Keypair,
    kad::{
        handler::KademliaHandlerProto, store::MemoryStore, Kademlia, KademliaConfig, KademliaEvent,
        QueryId, QueryResult,
    },
    mdns::{Mdns, MdnsConfig, MdnsEvent},
    multiaddr::Protocol,
    swarm::{
        behaviour::toggle::Toggle, ConnectionHandler, IntoConnectionHandler, NetworkBehaviour,
        NetworkBehaviourAction, PollParameters,
    },
    Multiaddr, PeerId,
};
use tracing::{info, warn};

pub const URSA_KAD_PROTOCOL: &[u8] = b"/ursa/kad/0.0.1";

pub struct PeerInfo {
    peer_id: PeerId,
    addresses: Vec<Multiaddr>,
}

#[derive(Debug)]
pub enum DiscoveryEvent {
    Connected(PeerId),
    Disconnected(PeerId),
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
    /// Optional MDNS protocol.
    mdns: Toggle<Mdns>,
}

impl DiscoveryBehaviour {
    pub fn new(keypair: &Keypair, config: &UrsaConfig) -> Self {
        let local_peer_id = PeerId::from(keypair.public());

        let bootstrap_nodes: Vec<(PeerId, Multiaddr)> = config
            .bootstrap_nodes
            .clone()
            .into_iter()
            .filter_map(|multiaddr| {
                let mut addr = multiaddr.to_owned();
                if let Some(Protocol::P2p(mh)) = addr.pop() {
                    let peer_id = PeerId::from_multihash(mh).unwrap();
                    Some((peer_id, addr))
                } else {
                    warn!("Could not parse bootstrap addr {}", multiaddr);
                    None
                }
            })
            .collect();

        // setup kademlia config
        let kademlia = {
            let store = MemoryStore::new(local_peer_id);
            // todo(botch): move replication factor to config
            // why 8?
            let replication_factor = NonZeroUsize::new(8).unwrap();

            let mut kad_config = KademliaConfig::default();
            kad_config
                .set_protocol_name(URSA_KAD_PROTOCOL)
                .set_replication_factor(replication_factor);

            Kademlia::with_config(local_peer_id, store, kad_config.clone())
        };

        let mdns = if config.mdns {
            Some(block_on(async {
                Mdns::new(MdnsConfig::default()).await.expect("mdns start")
            }))
        } else {
            None
        };

        Self {
            local_peer_id,
            kademlia,
            bootstrap_nodes,
            peers: HashSet::new(),
            peer_info: HashMap::new(),
            events: VecDeque::new(),
            mdns: mdns.into(),
        }
    }

    pub fn add_address(&mut self, peer_id: &PeerId, address: Multiaddr) {
        self.kademlia.add_address(peer_id, address);
    }

    pub fn peers(&self) -> &HashSet<PeerId> {
        &self.peers
    }

    pub fn peer_info(&self) -> &HashMap<PeerId, PeerInfo> {
        &self.peer_info
    }

    pub fn bootstrap(&mut self) -> Result<QueryId, Error> {
        for (peer_id, address) in self.bootstrap_addrs() {
            self.add_address(&peer_id, address.clone());
        }

        if self.bootstrap_nodes.is_empty() {
            return Err(anyhow!("No bootstrap nodes configured"));
        }

        info!("Bootstrapping with {:?}", self.bootstrap_nodes);

        // Ok(self.kademlia.get_closest_peers(self.local_peer_id))

        self.kademlia
            .bootstrap()
            .map_err(|err| anyhow!("{:?}", err))
    }

    pub fn bootstrap_addrs(&self) -> Vec<(PeerId, Multiaddr)> {
        self.bootstrap_nodes.clone()
    }

    fn handle_kad_event(&self, event: KademliaEvent) {
        info!("[KademliaEvent] {:?}", event);

        if let KademliaEvent::OutboundQueryCompleted { result, .. } = event {
            if let QueryResult::GetClosestPeers(Ok(closest_peers)) = result {
                let _peers = closest_peers.peers;
            }
        }
    }

    fn handle_mdns_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(discovered_peers) => {
                for (peer_id, address) in discovered_peers {
                    self.add_address(&peer_id, address)
                }
            }
            MdnsEvent::Expired(_) => {}
        }
    }
}

impl NetworkBehaviour for DiscoveryBehaviour {
    type ConnectionHandler = KademliaHandlerProto<QueryId>;

    type OutEvent = DiscoveryEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        self.kademlia.new_handler()
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        let mut addrs = Vec::new();
        addrs.extend(self.kademlia.addresses_of_peer(peer_id));
        addrs.extend(self.mdns.addresses_of_peer(peer_id));
        addrs
    }

    fn inject_connection_established(
        &mut self,
        peer_id: &PeerId,
        connection_id: &ConnectionId,
        endpoint: &ConnectedPoint,
        failed_addresses: Option<&Vec<Multiaddr>>,
        other_established: usize,
    ) {
        if self.peers.insert(*peer_id) {
            self.kademlia.inject_connection_established(
                peer_id,
                connection_id,
                endpoint,
                failed_addresses,
                other_established,
            );

            self.events.push_back(DiscoveryEvent::Connected(*peer_id));
        }
    }

    fn inject_connection_closed(
        &mut self,
        peer_id: &PeerId,
        connection_id: &ConnectionId,
        endpoint: &ConnectedPoint,
        handler: <Self::ConnectionHandler as IntoConnectionHandler>::Handler,
        remaining_established: usize,
    ) {
        if self.peers.remove(peer_id) {
            self.kademlia.inject_connection_closed(
                peer_id,
                connection_id,
                endpoint,
                handler,
                remaining_established,
            );

            self.events
                .push_back(DiscoveryEvent::Disconnected(*peer_id));
        }
    }

    fn inject_address_change(
        &mut self,
        peer_id: &PeerId,
        connection_id: &ConnectionId,
        old: &ConnectedPoint,
        new: &ConnectedPoint,
    ) {
        self.kademlia
            .inject_address_change(peer_id, connection_id, old, new);
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
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(NetworkBehaviourAction::GenerateEvent(event));
        }

        // Poll kademlia for events
        while let Poll::Ready(action) = self.kademlia.poll(cx, params) {
            match action {
                NetworkBehaviourAction::GenerateEvent(event) => self.handle_kad_event(event),
                NetworkBehaviourAction::Dial { opts, handler } => {
                    return Poll::Ready(NetworkBehaviourAction::Dial { opts, handler })
                }
                NetworkBehaviourAction::NotifyHandler {
                    peer_id,
                    handler,
                    event,
                } => {
                    return Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                        peer_id,
                        handler,
                        event,
                    })
                }
                NetworkBehaviourAction::ReportObservedAddr { address, score } => {
                    return Poll::Ready(NetworkBehaviourAction::ReportObservedAddr {
                        address,
                        score,
                    })
                }
                NetworkBehaviourAction::CloseConnection {
                    peer_id,
                    connection,
                } => {
                    return Poll::Ready(NetworkBehaviourAction::CloseConnection {
                        peer_id,
                        connection,
                    })
                }
            }
        }

        // Poll mdns for events
        while let Poll::Ready(action) = self.mdns.poll(cx, params) {
            match action {
                NetworkBehaviourAction::GenerateEvent(event) => self.handle_mdns_event(event),
                NetworkBehaviourAction::ReportObservedAddr { address, score } => {
                    return Poll::Ready(NetworkBehaviourAction::ReportObservedAddr {
                        address,
                        score,
                    })
                }
                NetworkBehaviourAction::CloseConnection {
                    peer_id,
                    connection,
                } => {
                    return Poll::Ready(NetworkBehaviourAction::CloseConnection {
                        peer_id,
                        connection,
                    })
                }
                NetworkBehaviourAction::Dial { .. }
                | NetworkBehaviourAction::NotifyHandler { .. } => {}
            }
        }

        Poll::Pending
    }
}
