use std::task::{Context, Poll};

use libp2p::{
    gossipsub::{Gossipsub, GossipsubEvent},
    identify::{Identify, IdentifyEvent},
    ping::{Ping, PingEvent},
    swarm::{
        NetworkBehaviour, NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters,
    },
    NetworkBehaviour,
};

use crate::discovery::behaviour::{DiscoveryBehaviour, DiscoveryEvent};
use tracing::{debug, error, trace, warn};

/// This is Fnet custome network behaviour that handles gossip, ping, identify, and discovery
/// This poll function must have the same signature as the NetworkBehaviour
/// function and will be called last within the generated NetworkBehaviour implementation.
#[derive(NetworkBehaviour)]
#[behaviour(
    out_event = "FnetBehaviourEvent",
    poll_method = "poll",
    event_process = true
)]
pub struct FnetBehaviour {
    ping: Ping,
    identify: Identify,
    gossipsub: Gossipsub,
    discovery: DiscoveryBehaviour,
}

impl FnetBehaviour {
    pub fn new() {
        // Setup the ping behaviour

        // Setup the identify behaviour

        // Setup the gossip behaviour

        // Setup the discovery behaviour
        todo!()
    }
    fn poll(
        &mut self,
        cx: &mut Context,
        _: &mut impl PollParameters,
    ) -> Poll<
        NetworkBehaviourAction<
            <Self as NetworkBehaviour>::OutEvent,
            <Self as NetworkBehaviour>::ConnectionHandler,
        >,
    > {
        todo!()
    }
    pub fn emit() {
        todo!()
    }
}

impl NetworkBehaviourEventProcess<PingEvent> for FnetBehaviour {
    fn inject_event(&mut self, event: PingEvent) {
        todo!()
    }
}

impl NetworkBehaviourEventProcess<IdentifyEvent> for FnetBehaviour {
    fn inject_event(&mut self, event: IdentifyEvent) {
        todo!()
    }
}

impl NetworkBehaviourEventProcess<GossipsubEvent> for FnetBehaviour {
    fn inject_event(&mut self, message: GossipsubEvent) {
        todo!()
    }
}

impl NetworkBehaviourEventProcess<DiscoveryEvent> for FnetBehaviour {
    fn inject_event(&mut self, event: DiscoveryEvent) {
        todo!()
    }
}

/// [FnetBehaviour]'s events
#[derive(Debug)]
pub enum FnetBehaviourEvent {
    Ping(PingEvent),
    Identify(IdentifyEvent),
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

impl From<GossipsubEvent> for FnetBehaviourEvent {
    fn from(event: GossipsubEvent) -> Self {
        Self::Gossip(event)
    }
}

impl From<DiscoveryEvent> for FnetBehaviourEvent {
    fn from(event: DiscoveryEvent) -> Self {
        Self::Discovery(event)
    }
}
