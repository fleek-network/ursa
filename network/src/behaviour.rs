//! # Ursa Behaviour implementation.
//!
//! Ursa custom behaviour implements [`NetworkBehaviour`] with the following options:
//!
//! - [`Ping`] A `NetworkBehaviour` that responds to inbound pings and
//! periodically sends outbound pings on every established connection.
//! - [`Identify`] A `Networkbehaviour` that automatically identifies nodes periodically, returns information
//! about them, and answers identify queries from other nodes.
//! - [`Bitswap`] A `Networkbehaviour` that handles sending and receiving blocks.
//! - [`Gossipsub`] A `Networkbehaviour` that handles the gossipsub protocol.
//! - [`DiscoveryBehaviour`]
//! - [`RequestResponse`] A `NetworkBehaviour` that implements a generic
//! request/response protocol or protocol family, whereby each request is
//! sent over a new substream on a connection.

use std::{
    collections::{HashSet, VecDeque},
    task::{Context, Poll},
    time::Duration,
};

use anyhow::{anyhow, Result};
use libipld::store::StoreParams;
use libp2p::{
    gossipsub::{
        error::{PublishError, SubscriptionError},
        Gossipsub, GossipsubConfigBuilder, GossipsubEvent, GossipsubMessage, IdentTopic as Topic,
        MessageAuthenticity, MessageId, PeerScoreParams, PeerScoreThresholds, ValidationMode,
    },
    identify::{Identify, IdentifyConfig, IdentifyEvent},
    kad::QueryId,
    ping::{Ping, PingEvent, PingFailure, PingSuccess},
    request_response::{
        ProtocolSupport, RequestResponse, RequestResponseConfig, RequestResponseEvent,
        RequestResponseMessage,
    },
    swarm::{
        NetworkBehaviour, NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters,
    },
    NetworkBehaviour, PeerId,
};
use libp2p_bitswap::{Bitswap, BitswapConfig, BitswapEvent, BitswapStore};
use tiny_cid::Cid;
use tracing::{debug, trace};

use crate::{
    codec::{UrsaExchangeCodec, UrsaExchangeProtocol, UrsaExchangeRequest, UrsaExchangeResponse},
    config::UrsaConfig,
    discovery::behaviour::{DiscoveryBehaviour, DiscoveryEvent},
    service::PROTOCOL_NAME,
};

/// Instead of storing the entire event we
/// can create a set of custom event types.
///
/// [Behaviour]'s events
#[derive(Debug)]
pub enum BehaviourEvent {
    PeerDiscovery(PeerId),
    PeerUnroutable(PeerId),
    Ping(PingEvent),
    Identify(IdentifyEvent),
    Bitswap(BitswapEvent),
    Gossip(GossipsubEvent),
    // add rpc events
    Discovery(DiscoveryEvent),
}

/// A `Networkbehaviour` that handles Ursa's different protocol implementations.
///
/// The poll function must have the same signature as the NetworkBehaviour
/// function and will be called last within the generated NetworkBehaviour implementation.
///
/// The events generated [`BehaviourEvent`].
#[derive(NetworkBehaviour)]
#[behaviour(
    out_event = "BehaviourEvent",
    poll_method = "poll",
    event_process = true
)]
pub struct Behaviour<P: StoreParams> {
    /// Aliving checks.
    ping: Ping,
    // Identifying peer info to other peers.
    identify: Identify,
    /// Bitswap for exchanging data between blocks between peers.
    bitswap: Bitswap<P>,
    /// Ursa's gossiping protocol for message propagation.
    gossipsub: Gossipsub,
    /// Kademlia discovery and bootstrap.
    discovery: DiscoveryBehaviour,
    /// request/response protocol implementation for [`UrsaExchangeProtocol`]
    request_response: RequestResponse<UrsaExchangeCodec>,
    /// Ursa's emitted events.
    #[behaviour(ignore)]
    events: VecDeque<BehaviourEvent>,
}

impl<P: StoreParams> Behaviour<P> {
    pub fn new<S: BitswapStore<Params = P>>(config: &UrsaConfig, store: S) -> Self {
        let local_public_key = config.keypair.public();

        // TODO: check if UrsaConfig has configs for the behaviours, if not instaniate new ones

        // Setup the ping behaviour
        let ping = Ping::default();

        // Setup the bitswap behaviour
        let bitswap = Bitswap::new(BitswapConfig::new(), store);

        // Setup the identify behaviour
        let identify = Identify::new(IdentifyConfig::new(PROTOCOL_NAME.into(), local_public_key));

        // Setup the discovery behaviour
        let discovery =
            DiscoveryBehaviour::new(&config).with_bootstrap_nodes(config.bootstrap_nodes.clone());

        let request_response = {
            let protocols = std::iter::once((UrsaExchangeProtocol {}, ProtocolSupport::Full));

            let cfg = RequestResponseConfig::default();
            // Todo: set using config
            cfg.set_connection_keep_alive(Duration::from_secs(10));
            cfg.set_request_timeout(todo!());

            RequestResponse::new(UrsaExchangeCodec, protocols, cfg)
        };

        // Setup the gossip behaviour
        // move to config
        // based on node v0 spec
        let gossipsub = {
            let history_length = 5;
            let history_gossip = 3;
            let mesh_n = 8;
            let mesh_n_low = 4;
            let mesh_n_high = 12;
            let retain_scores = 4;
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
                Gossipsub::new(MessageAuthenticity::Signed(config.key), gossip_config)
                    .map_err(|err| anyhow!("{}", err));

            // Defaults for now
            let params = PeerScoreParams::default();
            let threshold = PeerScoreThresholds::defaults();

            gossipsub.with_peer_score(params, threshold).unwrap()
        };

        Behaviour {
            ping,
            bitswap,
            identify,
            gossipsub,
            discovery,
            // todo rpc
            request_response,
            events: VecDeque::new(),
        }
    }

    pub fn peers(&mut self) -> HashSet<PeerId> {
        self.discovery.peers()
    }

    pub fn bootstrap(&mut self) -> Result<QueryId, String> {
        self.discovery.bootstrap()
    }

    pub fn subscribe(&mut self, topic: &Topic) -> Result<bool, SubscriptionError> {
        self.gossipsub.subscribe(topic)
    }

    pub fn unsubscribe(&mut self, topic: &Topic) -> Result<bool, PublishError> {
        self.gossipsub.unsubscribe(topic)
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
        match self.events.pop_front() {
            Some(event) => Poll::Ready(NetworkBehaviourAction::GenerateEvent(event)),
            None => todo!(),
            _ => Poll::Pending,
        }
    }
}

impl<P: StoreParams> NetworkBehaviourEventProcess<PingEvent> for Behaviour<P> {
    fn inject_event(&mut self, event: PingEvent) {
        let peer = event.peer.to_base58();

        match event.result {
            Ok(result) => match result {
                PingSuccess::Pong => {
                    trace!(
                        "PingSuccess::Pong received a ping and sent back a pong to {}",
                        peer
                    );
                }
                PingSuccess::Ping { rtt } => {
                    trace!(
                        "PingSuccess::Ping with rtt {} from {} in ms",
                        rtt.as_millis(),
                        peer
                    );
                }
            },
            Err(err) => {
                match err {
                    PingFailure::Timeout => {
                        debug!(
                            "PingFailure::Timeout no response was received from {}",
                            peer
                        );
                        // remove peer from list of connected.
                    }
                    PingFailure::Unsupported => {
                        debug!("PingFailure::Unsupported the peer {} does not support the ping protocol", peer);
                    }
                    PingFailure::Other { error } => {
                        debug!(
                            "PingFailure::Other the ping failed with {} for reasons {}",
                            peer, error
                        );
                    }
                }
            }
        }
    }
}

impl<P: StoreParams> NetworkBehaviourEventProcess<IdentifyEvent> for Behaviour<P> {
    fn inject_event(&mut self, event: IdentifyEvent) {
        match event {
            IdentifyEvent::Received { peer_id, info } => {
                trace!(
                    "Identification information {} has been received from a peer {}.",
                    info,
                    peer_id
                );
                // Identification information has been received from a peer.
                // handle identity and add to the list of peers
            }
            IdentifyEvent::Sent { .. }
            | IdentifyEvent::Pushed { .. }
            | IdentifyEvent::Error { .. } => {}
        }
    }
}

impl<P: StoreParams> NetworkBehaviourEventProcess<GossipsubEvent> for Behaviour<P> {
    fn inject_event(&mut self, event: GossipsubEvent) {
        match event {
            GossipsubEvent::Message {
                propagation_source,
                message_id,
                message,
            } => {
                if let Ok(cid) = Cid::try_from(message.data) {
                    self.events.push_back(event.into());
                }
            }
            GossipsubEvent::Subscribed { peer_id, topic } => {
                // A remote subscribed to a topic.
                // subscribe to new topic.
            }
            GossipsubEvent::Unsubscribed { peer_id, topic } => {
                // A remote unsubscribed from a topic.
                // remove subscription.
            }
            GossipsubEvent::GossipsubNotSupported { peer_id } => {
                // A peer that does not support gossipsub has connected.
                // the scoring/rating should happen here.
                // disconnect.
            }
        }
    }
}

impl<P: StoreParams> NetworkBehaviourEventProcess<BitswapEvent> for Behaviour<P> {
    fn inject_event(&mut self, event: BitswapEvent) {
        match event {
            BitswapEvent::Progress(query_id, counter) => {
                // Received a block from a peer. Includes the number of known missing blocks for a sync query.
                // When a block is received and missing blocks is not empty the counter is increased.
                // If missing blocks is empty the counter is decremented.

                // keep track of all the query ids.
            }
            BitswapEvent::Complete(query_id, result) => {
                // A get or sync query completed.
            }
        }
    }
}

impl<P: StoreParams> NetworkBehaviourEventProcess<DiscoveryEvent> for Behaviour<P> {
    fn inject_event(&mut self, event: DiscoveryEvent) {
        match event {
            DiscoveryEvent::Discoverd(peer_id) => todo!(),
            DiscoveryEvent::UnroutablePeer(_) => todo!(),
        }
    }
}

impl<P: StoreParams>
    NetworkBehaviourEventProcess<RequestResponseEvent<UrsaExchangeRequest, UrsaExchangeResponse>>
    for Behaviour<P>
{
    fn inject_event(
        &mut self,
        event: RequestResponseEvent<UrsaExchangeRequest, UrsaExchangeResponse>,
    ) {
        match event {
            RequestResponseEvent::Message { peer, message } => match message {
                RequestResponseMessage::Request {
                    request_id,
                    request,
                    channel,
                } => todo!(),
                RequestResponseMessage::Response {
                    request_id,
                    response,
                } => todo!(),
            },
            RequestResponseEvent::OutboundFailure {
                peer,
                request_id,
                error,
            } => todo!(),
            RequestResponseEvent::InboundFailure {
                peer,
                request_id,
                error,
            } => todo!(),
            RequestResponseEvent::ResponseSent { peer, request_id } => todo!(),
        }
    }
}

// ToDo: rpc event
// impl<P: StoreParams> NetworkBehaviourEventProcess<RPCEvent> for Behaviour<P> {
//     fn inject_event(&mut self, event: RPCEvent) {
//         todo!()
//     }
// }
