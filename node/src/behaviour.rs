//! Fnet Behaviour implementation.
//!
//!
//!

use std::{
    collections::VecDeque,
    task::{Context, Poll},
    time::Duration,
};

use libipld::store::StoreParams;
use libp2p::{
    gossipsub::{
        error::SubscriptionError, Gossipsub, GossipsubConfigBuilder, GossipsubEvent,
        GossipsubMessage, IdentTopic as Topic, MessageAuthenticity, MessageId, PeerScoreParams,
        PeerScoreThresholds, ValidationMode,
    },
    identify::{Identify, IdentifyConfig, IdentifyEvent},
    kad::QueryId,
    ping::{Ping, PingEvent, PingFailure, PingSuccess},
    swarm::{
        NetworkBehaviour, NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters,
    },
    NetworkBehaviour,
};
use libp2p_bitswap::{Bitswap, BitswapConfig, BitswapEvent, BitswapStore};

use crate::{
    config::FnetConfig,
    discovery::behaviour::{DiscoveryBehaviour, DiscoveryEvent},
    service::PROTOCOL_NAME,
};

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

/// This is Fnet's custom network behaviour that handles
/// all the [`Ping`], [`Identify`], [`Bitswap`], [`Gossipsub`], and [`DiscoveryBehaviour`].
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
    /// Aliving checks.
    ping: Ping,
    // Identifying peer info to other peers.
    identify: Identify,
    ///
    bitswap: Bitswap<P>,
    /// Fnet's gossiping protocol for message propagation.
    gossipsub: Gossipsub,
    /// Kademlia discovery and bootstrap.
    discovery: DiscoveryBehaviour,

    /// Fleek Network list of emitted events.
    #[behaviour(ignore)]
    events: VecDeque<FnetBehaviourEvent>,
}

impl<P: StoreParams> FnetBehaviour<P> {
    pub fn new<S: BitswapStore<Params = P>>(config: &FnetConfig, store: S) -> Self {
        let local_public_key = config.keypair.public();

        //TODO: check if FnetConfig has configs for the behaviours, if not instaniate new ones

        // Setup the ping behaviour
        let ping = Ping::default();

        // Setup the bitswap behaviour
        let bitswap = Bitswap::new(BitswapConfig::new(), store);

        // Setup the identify behaviour
        let identify = Identify::new(IdentifyConfig::new(PROTOCOL_NAME.into(), local_public_key));

        // Setup the discovery behaviour
        let discovery =
            DiscoveryBehaviour::new(&config).with_bootstrap_nodes(config.bootstrap_nodes.clone());

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

        FnetBehaviour {
            ping,
            bitswap,
            identify,
            gossipsub,
            discovery,
            events: vec![],
        }
    }

    pub fn bootstrap(&mut self) -> Result<QueryId, String> {
        self.discovery.bootstrap()
    }

    pub fn subscribe(&mut self, topic: &Topic) -> Result<bool, SubscriptionError> {
        self.gossipsub.subscribe(topic)
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
        match event.result {
            Ok(result) => match result {
                PingSuccess::Pong => todo!(),
                PingSuccess::Ping { rtt } => todo!(),
            },
            Err(err) => match err {
                PingFailure::Timeout => todo!(),
                PingFailure::Unsupported => todo!(),
                PingFailure::Other { error } => todo!(),
            },
        }
    }
}

impl<P: StoreParams> NetworkBehaviourEventProcess<IdentifyEvent> for FnetBehaviour<P> {
    fn inject_event(&mut self, event: IdentifyEvent) {
        match event {
            IdentifyEvent::Received { peer_id, info } => todo!(),
            IdentifyEvent::Sent { peer_id } => todo!(),
            IdentifyEvent::Pushed { peer_id } => todo!(),
            IdentifyEvent::Error { peer_id, error } => todo!(),
        }
    }
}

impl<P: StoreParams> NetworkBehaviourEventProcess<GossipsubEvent> for FnetBehaviour<P> {
    fn inject_event(&mut self, message: GossipsubEvent) {
        match message {
            GossipsubEvent::Message {
                propagation_source,
                message_id,
                message,
            } => todo!(),
            GossipsubEvent::Subscribed { peer_id, topic } => todo!(),
            GossipsubEvent::Unsubscribed { peer_id, topic } => todo!(),
            GossipsubEvent::GossipsubNotSupported { peer_id } => todo!(),
        }
    }
}

impl<P: StoreParams> NetworkBehaviourEventProcess<BitswapEvent> for FnetBehaviour<P> {
    fn inject_event(&mut self, event: BitswapEvent) {
        match event {
            BitswapEvent::Progress(query_id, counter) => todo!(),
            BitswapEvent::Complete(query_id, result) => todo!(),
        }
    }
}

impl<P: StoreParams> NetworkBehaviourEventProcess<DiscoveryEvent> for FnetBehaviour<P> {
    fn inject_event(&mut self, event: DiscoveryEvent) {
        todo!()
    }
}
