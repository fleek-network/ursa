use crate::identify::PEERS;
use crate::Recorder;
use libp2p_core::{ConnectedPoint, PeerId};
use libp2p_swarm::SwarmEvent;
use metrics::Label;
use metrics::{decrement_gauge, increment_counter};

impl<TBvEv, THandleErr> Recorder for SwarmEvent<TBvEv, THandleErr> {
    fn record(&self) {
        match self {
            SwarmEvent::Behaviour(_) => {}
            SwarmEvent::ConnectionEstablished { endpoint, .. } => {
                increment_counter!(
                    "swarm_connections_established",
                    vec![Role::from(endpoint.clone()).into()]
                );
            }
            SwarmEvent::ConnectionClosed {
                endpoint,
                peer_id,
                num_established,
                ..
            } => {
                increment_counter!(
                    "swarm_connections_closed",
                    vec![Role::from(endpoint.clone()).into()]
                );

                // If the last connection to a peer is closed, decrement the protocols identified by them
                if *num_established == 0 {
                    let mut peers = PEERS.write().unwrap();
                    if let Some(protocols) = peers.remove(peer_id) {
                        for protocol in protocols {
                            decrement_gauge!(
                            "identify_supported_protocols",
                            1.0,
                            vec![Label::new("protocol", protocol.clone())]
                        );
                        }
                    }
                }
            }
            SwarmEvent::IncomingConnection { .. } => {
                increment_counter!("swarm_connections_incoming");
            }
            SwarmEvent::IncomingConnectionError { error, .. } => {
                increment_counter!(
                    "swarm_connections_incoming_error",
                    vec![Label::new("error", error.to_string()),]
                );
            }
            SwarmEvent::OutgoingConnectionError { error, peer_id } => {
                increment_counter!(
                    "swarm_outgoing_connection_error",
                    vec![
                        Label::new("error", error.to_string()),
                        PeerStatus::from(peer_id).into()
                    ]
                );
            }
            SwarmEvent::BannedPeer { .. } => {
                increment_counter!("swarm_connected_to_banned_peer");
            }
            SwarmEvent::NewListenAddr { .. } => {
                increment_counter!("swarm_new_listen_addr");
            }
            SwarmEvent::ExpiredListenAddr { .. } => {
                increment_counter!("swarm_expired_listen_addr");
            }
            SwarmEvent::ListenerClosed { .. } => {
                increment_counter!("swarm_listener_closed");
            }
            SwarmEvent::ListenerError { .. } => {
                increment_counter!("swarm_listener_error");
            }
            SwarmEvent::Dialing(_) => {
                increment_counter!("swarm_dial_attempt");
            }
        }
    }
}

#[derive(Hash, Clone, Eq, PartialEq, Copy)]
enum PeerStatus {
    Known,
    Unknown,
}

impl From<PeerStatus> for Label {
    fn from(status: PeerStatus) -> Self {
        match status {
            PeerStatus::Known => Label::new("peer_status", "known"),
            PeerStatus::Unknown => Label::new("peer_status", "unknown"),
        }
    }
}

impl From<&Option<PeerId>> for PeerStatus {
    fn from(peer: &Option<PeerId>) -> Self {
        match peer {
            Some(_) => PeerStatus::Known,
            None => PeerStatus::Unknown,
        }
    }
}

pub enum Role {
    Dialer,
    Listener,
}

impl From<ConnectedPoint> for Role {
    fn from(point: ConnectedPoint) -> Self {
        match point {
            ConnectedPoint::Dialer { .. } => Role::Dialer,
            ConnectedPoint::Listener { .. } => Role::Listener,
        }
    }
}

impl From<Role> for Label {
    fn from(role: Role) -> Self {
        match role {
            Role::Dialer => Label::new("role", "dialer"),
            Role::Listener => Label::new("role", "listener"),
        }
    }
}
