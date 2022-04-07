use crate::discovery::{DiscoveryBehaviour, DiscoveryEvent};
use libp2p::{
    gossipsub::{Gossipsub, GossipsubEvent},
    identify::IdentifyEvent,
    kad::KademliaEvent,
    ping::PingEvent,
    swarm::{NetworkBehaviour, NetworkBehaviourEventProcess},
    NetworkBehaviour,
};

#[derive(Debug)]
pub enum FnetBehaviourEvent {
    Ping(PingEvent),
    Identify(IdentifyEvent),
    Kademlia(KademliaEvent),
    Discovery(DiscoveryEvent),
    Gossip(GossipsubEvent),
}

impl From<PingEvent> for FnetBehaviourEvent {
    fn from(event: PingEvent) -> Self {
        Self::Ping(event)
    }
}

impl From<IdentifyEvent> for FnetBehaviourEvent {
    fn from(event: IdentifyEvent) -> Self {
        Self::Identify(event)
    }
}

impl From<KademliaEvent> for FnetBehaviourEvent {
    fn from(event: KademliaEvent) -> Self {
        Self::Kademlia(event)
    }
}

impl From<DiscoveryEvent> for FnetBehaviourEvent {
    fn from(event: DiscoveryEvent) -> Self {
        Self::Discovery(event)
    }
}

impl From<GossipsubEvent> for FnetBehaviourEvent {
    fn from(event: GossipsubEvent) -> Self {
        Self::Gossip(event)
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "FnetBehaviourEvent", poll_method = "poll")]
pub struct FnetBehaviour {
    gossipsub: Gossipsub,
    discovery: DiscoveryBehaviour,
}

impl FnetBehaviour {
    pub fn new() {
        todo!()
    }
    pub fn poll() {
        todo!()
    }
    pub fn emit() {
        todo!()
    }
}

impl NetworkBehaviourEventProcess<IdentifyEvent> for FnetBehaviour {
    fn inject_event(&mut self, event: IdentifyEvent) {
        todo!()
    }
}
