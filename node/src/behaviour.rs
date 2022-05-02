//! Fnet Behaviour implementation.
//!
//!
//!

use std::{
    task::{Context, Poll},
    time::Duration,
};

use libipld::store::StoreParams;
use libp2p::{
    gossipsub::{
        Gossipsub, GossipsubConfigBuilder, GossipsubEvent, GossipsubMessage, MessageAuthenticity,
        MessageId, PeerScoreParams, PeerScoreThresholds, ValidationMode,
    },
    identify::{Identify, IdentifyConfig, IdentifyEvent},
    ping::{Ping, PingEvent},
    swarm::{
        NetworkBehaviour, NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters,
    },
    NetworkBehaviour,
};
use libp2p_bitswap::{Bitswap, BitswapConfig, BitswapStore};

use crate::{
    config::FnetConfig,
    discovery::behaviour::{DiscoveryBehaviour, DiscoveryEvent},
};

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
pub struct FnetBehaviour<P: StoreParams> {
    ping: Ping,
    // todo add rpc
    identify: Identify,
    bitswap: Bitswap<P>,
    gossipsub: Gossipsub,
    discovery: DiscoveryBehaviour,
}

impl<P: StoreParams> FnetBehaviour<P> {
    pub fn new<S: BitswapStore<Params = P>>(config: &FnetConfig, store: S) -> Self {
        let local_public_key = config.key.public();
        let protocol_version = "fnet/0.0.1".into();

        //TODO: check if FnetConfig has configs for the behaviours, if not instaniate new ones

        // Setup the ping behaviour
        let ping = Ping::default();

        // Setup the bitswap behaviour
        let bitswap = Bitswap::new(BitswapConfig::new(), store);

        // Setup the identify behaviour
        let identify = Identify::new(IdentifyConfig::new(protocol_version, local_public_key));

        // Setup the gossip behaviour
        // move to config
        // based on node v0 spec
        let gossipsub = {
            let history_length = 5;
            let history_gossip = 3;
            let mesh_n = 8;
            let mesh_n_low = 4;
            let mesh_n_high = 12;
            let gossip_lazy = mesh_n;
            let heartbeat_interval = Duration::from_secs(1);
            let fanout_ttl = Duration::from_secs(60);
            // D_out
            let mesh_outbound_min = (mesh_n / 2) - 1;
            let max_transmit_size = 1;
            let max_msgs_per_rpc = 1;
            let cache_size = 1;
            let id_fn = move |message: &GossipsubMessage| MessageId::from(todo!());

            let gossip_config = GossipsubConfigBuilder::default()
                .history_length(history_length)
                .history_gossip(history_gossip)
                .mesh_n(mesh_n)
                .mesh_n_low(mesh_n_low)
                .mesh_n_high(mesh_n_high)
                // .retain_scores(retain_scores)
                .gossip_lazy(gossip_lazy)
                .heartbeat_interval(heartbeat_interval)
                .fanout_ttl(fanout_ttl)
                .max_transmit_size(max_transmit_size)
                .duplicate_cache_time(cache_size)
                .validate_messages()
                .validation_mode(ValidationMode::Strict)
                .message_id_fn(id_fn)
                .allow_self_origin(true)
                .mesh_outbound_min(mesh_outbound_min)
                .max_messages_per_rpc(max_msgs_per_rpc)
                .build()
                .expect("gossipsub config");

            let mut gossipsub =
                Gossipsub::new(MessageAuthenticity::Signed(config.key), gossip_config).unwrap();

            // Defaults for now
            let params = PeerScoreParams::default();
            let threshold = PeerScoreThresholds::defaults();

            gossipsub.with_peer_score(params, threshold).unwrap()
        };

        // Setup the discovery behaviour
        let discovery = DiscoveryBehaviour::new(local_public_key);

        FnetBehaviour {
            ping,
            bitswap,
            identify,
            gossipsub,
            discovery,
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

impl<P: StoreParams> NetworkBehaviourEventProcess<PingEvent> for FnetBehaviour<P> {
    fn inject_event(&mut self, event: PingEvent) {
        todo!()
    }
}

impl<P: StoreParams> NetworkBehaviourEventProcess<IdentifyEvent> for FnetBehaviour<P> {
    fn inject_event(&mut self, event: IdentifyEvent) {
        todo!()
    }
}

impl<P: StoreParams> NetworkBehaviourEventProcess<GossipsubEvent> for FnetBehaviour<P> {
    fn inject_event(&mut self, message: GossipsubEvent) {
        todo!()
    }
}

impl<P: StoreParams> NetworkBehaviourEventProcess<DiscoveryEvent> for FnetBehaviour<P> {
    fn inject_event(&mut self, event: DiscoveryEvent) {
        todo!()
    }
}

/// [FnetBehaviour]'s events
#[derive(Debug)]
pub enum FnetBehaviourEvent {
    Ping(PingEvent),
    Gossip(GossipsubEvent),
    Identify(IdentifyEvent),
    // add bitswap and rpc events
    Discovery(DiscoveryEvent),
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
