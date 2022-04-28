//! Fnet Behaviour implementation.
//!
//!
//!

use std::task::{Context, Poll};

use libp2p::{
    gossipsub::{Gossipsub, GossipsubEvent},
    identify::{Identify, IdentifyEvent},
    identity::Keypair,
    ping::{Ping, PingEvent},
    swarm::{
        NetworkBehaviour, NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters,
    },
    NetworkBehaviour,
};

use crate::discovery::behaviour::{DiscoveryBehaviour, DiscoveryEvent};

/// This is Fnet custom network behaviour that handles
/// [`Gossipsub`], [`Ping`], [`Identify`], and [`DiscoveryBehaviour`].
///
/// The poll function must have the same signature as the NetworkBehaviour
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

// 20000000
// dfx canister --network ic call tgodh-faaaa-aaaab-qaefa-cai approve '(principal "tpni3-tiaaa-aaaab-qaeeq-cai", 20000000:nat)'
// dfx canister --network ic call tpni3-tiaaa-aaaab-qaeeq-cai burn '(principal "fle2e-ltcun-tpi5w-25chp-byb56-dfl72-f664t-slvy", 20000000:nat)'
// 0x011478794f516fb7d9d3016a36fdcdbd5121171c2e5199df712d7a8399138553
// 0x60DC1a1FD50F1cdA1D44dFf69Cec3E5C065417e8

impl FnetBehaviour {
    pub fn new(keypair: &Keypair) -> Self {
        // Setup the ping behaviour

        // Setup the identify behaviour

        // Setup the gossip behaviour

        // Setup the discovery behaviour

        FnetBehaviour {
            ping: todo!(),
            identify: todo!(),
            gossipsub: todo!(),
            discovery: todo!(),
        }
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
