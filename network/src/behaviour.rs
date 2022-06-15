//! # Ursa Behaviour implementation.
//!
//! Ursa custom behaviour implements [`NetworkBehaviour`] with the following options:
//!
//! - [`Ping`] A `NetworkBehaviour` that responds to inbound pings and
//!   periodically sends outbound pings on every established connection.
//! - [`Identify`] A `Networkbehaviour` that automatically identifies nodes periodically, returns information
//!   about them, and answers identify queries from other nodes.
//! - [`Bitswap`] A `Networkbehaviour` that handles sending and receiving blocks.
//! - [`Gossipsub`] A `Networkbehaviour` that handles the gossipsub protocol.
//! - [`DiscoveryBehaviour`]
//! - [`RequestResponse`] A `NetworkBehaviour` that implements a generic
//!   request/response protocol or protocol family, whereby each request is
//!   sent over a new substream on a connection.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    iter,
    task::{Context, Poll},
    time::Duration,
};

use anyhow::{Error, Result};
use futures::channel::oneshot;
use libipld::store::StoreParams;
use libp2p::{
    core::either::EitherError,
    gossipsub::{
        error::{GossipsubHandlerError, PublishError, SubscriptionError},
        Gossipsub, GossipsubEvent, GossipsubMessage, IdentTopic as Topic, MessageId,
        PeerScoreParams, PeerScoreThresholds, TopicHash,
    },
    identify::{Identify, IdentifyConfig, IdentifyEvent},
    identity::Keypair,
    kad::{KademliaEvent, QueryId},
    ping::{self, Ping, PingEvent, PingFailure, PingSuccess},
    request_response::{
        ProtocolSupport, RequestId, RequestResponse, RequestResponseConfig, RequestResponseEvent,
        RequestResponseMessage, ResponseChannel,
    },
    swarm::{
        ConnectionHandlerUpgrErr, NetworkBehaviour, NetworkBehaviourAction,
        NetworkBehaviourEventProcess, PollParameters,
    },
    NetworkBehaviour, PeerId,
};
use libp2p_bitswap::{Bitswap, BitswapConfig, BitswapEvent, BitswapStore};
use tracing::{debug, trace, warn};

use crate::{
    codec::protocol::{UrsaExchangeCodec, UrsaExchangeRequest, UrsaExchangeResponse, UrsaProtocol},
    config::UrsaConfig,
    discovery::{DiscoveryBehaviour, DiscoveryEvent},
    gossipsub::UrsaGossipsub,
};

pub const IPFS_PROTOCOL: &str = "ipfs/0.1.0";

/// [Behaviour]'s events
/// Requests and failure events emitted by the `NetworkBehaviour`.
#[derive(Debug)]
pub enum BehaviourEvent {
    Bitswap(BitswapEvent),
    /// An event trigger when remote peer connects.
    PeerConnected(PeerId),
    /// An event trigger when remote peer disconnects.
    PeerDisconnected(PeerId),
    /// A Gossip message request was recieved from a peer.
    GossipMessage {
        peer: PeerId,
        topic: TopicHash,
        message: GossipsubMessage,
    },
    /// A message request was recieved from a peer.
    /// Attached is a channel for returning a response.
    RequestMessage {
        peer: PeerId,
        request: UrsaExchangeRequest,
        channel: ResponseChannel<UrsaExchangeResponse>,
    },
}

pub type BehaviourEventError = EitherError<
    EitherError<
        EitherError<
            EitherError<
                EitherError<ping::Failure, std::io::Error>,
                ConnectionHandlerUpgrErr<std::io::Error>,
            >,
            GossipsubHandlerError,
        >,
        std::io::Error,
    >,
    ConnectionHandlerUpgrErr<std::io::Error>,
>;

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

    /// request/response protocol implementation for [`UrsaProtocol`]
    request_response: RequestResponse<UrsaExchangeCodec>,

    /// Ursa's emitted events.
    #[behaviour(ignore)]
    events: VecDeque<BehaviourEvent>,

    /// Pending responses
    #[behaviour(ignore)]
    pending_requests: HashMap<RequestId, ResponseChannel<UrsaExchangeResponse>>,

    /// Pending requests
    #[behaviour(ignore)]
    pending_responses: HashMap<RequestId, oneshot::Sender<Result<UrsaExchangeResponse>>>,
}

impl<P: StoreParams> Behaviour<P> {
    pub fn new<S: BitswapStore<Params = P>>(
        keypair: &Keypair,
        config: &UrsaConfig,
        bitswap_store: S,
    ) -> Self {
        let local_public_key = keypair.public();

        // Setup the ping behaviour
        let ping = Ping::default();

        // Setup the gossip behaviour
        let mut gossipsub = UrsaGossipsub::new(keypair, config);
        // todo(botch): handle gracefully
        gossipsub
            .with_peer_score(PeerScoreParams::default(), PeerScoreThresholds::default())
            .expect("PeerScoreParams and PeerScoreThresholds");

        // Setup the discovery behaviour
        let discovery = DiscoveryBehaviour::new(keypair, config);

        // Setup the bitswap behaviour
        let bitswap = Bitswap::new(BitswapConfig::default(), bitswap_store);

        // Setup the identify behaviour
        let identify = Identify::new(IdentifyConfig::new(IPFS_PROTOCOL.into(), local_public_key));

        let request_response = {
            let mut cfg = RequestResponseConfig::default();

            // todo(botch): calculate an upper limit to allow for large files
            cfg.set_request_timeout(Duration::from_secs(60));

            let protocols = iter::once((UrsaProtocol, ProtocolSupport::Full));

            RequestResponse::new(UrsaExchangeCodec, protocols, cfg)
        };

        Behaviour {
            ping,
            bitswap,
            identify,
            gossipsub,
            discovery,
            request_response,
            events: VecDeque::new(),
            pending_requests: HashMap::default(),
            pending_responses: HashMap::default(),
        }
    }

    pub fn publish(
        &mut self,
        topic: Topic,
        data: GossipsubMessage,
    ) -> Result<MessageId, PublishError> {
        self.gossipsub.publish(topic, data.data)
    }

    pub fn peers(&self) -> HashSet<PeerId> {
        self.discovery.peers().clone()
    }

    pub fn bootstrap(&mut self) -> Result<QueryId, Error> {
        self.discovery.bootstrap()
    }

    pub fn subscribe(&mut self, topic: &Topic) -> Result<bool, SubscriptionError> {
        self.gossipsub.subscribe(topic)
    }

    pub fn unsubscribe(&mut self, topic: &Topic) -> Result<bool, PublishError> {
        self.gossipsub.unsubscribe(topic)
    }

    pub fn send_request(
        &mut self,
        peer: PeerId,
        request: UrsaExchangeRequest,
        sender: oneshot::Sender<Result<UrsaExchangeResponse>>,
    ) -> Result<()> {
        let request_id = self.request_response.send_request(&peer, request);
        self.pending_responses.insert(request_id, sender);

        Ok(())
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
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(NetworkBehaviourAction::GenerateEvent(event));
        }

        Poll::Pending
    }

    fn handle_ping(&mut self, event: PingEvent) {
        let peer = event.peer.to_base58();

        match event.result {
            Ok(result) => match result {
                PingSuccess::Pong => {
                    trace!(
                        "PingSuccess::Pong] - received a ping and sent back a pong to {}",
                        peer
                    );
                }
                PingSuccess::Ping { rtt } => {
                    trace!(
                        "[PingSuccess::Ping] - with rtt {} from {} in ms",
                        rtt.as_millis(),
                        peer
                    );
                    // perhaps we can set rtt for each peer
                }
            },
            Err(err) => {
                match err {
                    PingFailure::Timeout => {
                        debug!(
                            "[PingFailure::Timeout] - no response was received from {}",
                            peer
                        );
                        // remove peer from list of connected.
                    }
                    PingFailure::Unsupported => {
                        debug!("[PingFailure::Unsupported] - the peer {} does not support the ping protocol", peer);
                    }
                    PingFailure::Other { error } => {
                        debug!(
                            "[PingFailure::Other] - the ping failed with {} for reasons {}",
                            peer, error
                        );
                    }
                }
            }
        }
    }

    fn handle_identify(&mut self, event: IdentifyEvent) {
        match event {
            IdentifyEvent::Received { peer_id, info } => {
                trace!(
                    "[IdentifyEvent::Received] - with version {} has been received from a peer {}.",
                    info.protocol_version,
                    peer_id
                );

                if self.peers().contains(&peer_id) {
                    trace!(
                        "[IdentifyEvent::Received] - peer {} already known!",
                        peer_id
                    );
                    ()
                }

                // check if received identify is from a peer on the same network
                if info
                    .protocols
                    .iter()
                    .any(|name| name.as_bytes() == IPFS_PROTOCOL.as_bytes())
                {
                    self.gossipsub.add_explicit_peer(&peer_id);

                    for address in info.listen_addrs {
                        self.discovery.add_address(&peer_id, address.clone());
                        self.request_response.add_address(&peer_id, address.clone());
                    }
                }
            }
            IdentifyEvent::Sent { .. }
            | IdentifyEvent::Pushed { .. }
            | IdentifyEvent::Error { .. } => {}
        }
    }

    fn handle_bitswap(&mut self, event: BitswapEvent) {
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

    fn handle_gossipsub(&mut self, event: GossipsubEvent) {
        match event {
            GossipsubEvent::Message {
                propagation_source,
                message,
                ..
            } => {
                self.events.push_back(BehaviourEvent::GossipMessage {
                    peer: propagation_source,
                    topic: message.topic.clone(),
                    message,
                });
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

    fn handle_discovery(&mut self, event: DiscoveryEvent) {
        match event {
            DiscoveryEvent::Connected(peer_id) => {
                self.events
                    .push_back(BehaviourEvent::PeerConnected(peer_id));
            }
            DiscoveryEvent::Disconnected(peer_id) => {
                self.events
                    .push_back(BehaviourEvent::PeerDisconnected(peer_id));
            }
        }
    }

    fn handle_request_response(
        &mut self,
        event: RequestResponseEvent<UrsaExchangeRequest, UrsaExchangeResponse>,
    ) {
        match event {
            RequestResponseEvent::Message { peer, message } => {
                match message {
                    RequestResponseMessage::Request {
                        request_id,
                        request,
                        channel,
                    } => {
                        debug!(
                            "[RequestResponseMessage::Request] - {} {}: {:?}",
                            request_id, peer, request
                        );
                        // self.pending_requests.insert(request_id, channel);

                        self.events.push_back(BehaviourEvent::RequestMessage {
                            peer,
                            request,
                            channel,
                        });
                    }
                    RequestResponseMessage::Response {
                        request_id,
                        response,
                    } => {
                        debug!(
                            "[RequestResponseMessage::Response] - {} {}: {:?}",
                            request_id, peer, response
                        );

                        if let Some(request) = self.pending_responses.remove(&request_id) {
                            if request.send(Ok(response)).is_err() {
                                warn!("[RequestResponseMessage::Response] - failed to send request: {:?}", request_id);
                            }
                        }

                        debug!("[RequestResponseMessage::Response] - failed to remove channel for: {:?}", request_id);
                    }
                }
            }
            RequestResponseEvent::OutboundFailure {
                peer,
                request_id,
                error,
            } => {
                debug!(
                    "[RequestResponseMessage::OutboundFailure] - {} {}: {:?}",
                    peer.to_string(),
                    request_id.to_string(),
                    error.to_string()
                );

                if let Some(request) = self.pending_responses.remove(&request_id) {
                    if request.send(Err(error.into())).is_err() {
                        warn!("[RequestResponseMessage::OutboundFailure] - failed to send request: {:?}", request_id);
                    }
                }

                debug!("[RequestResponseMessage::OutboundFailure] - failed to remove channel for: {:?}", request_id);
            }
            RequestResponseEvent::InboundFailure {
                peer,
                request_id,
                error,
            } => {
                warn!(
                    "[RequestResponseMessage::InboundFailure] - {} {}: {:?}",
                    peer.to_string(),
                    request_id.to_string(),
                    error.to_string()
                );
            }
            RequestResponseEvent::ResponseSent { peer, request_id } => {
                debug!(
                    "[RequestResponseMessage::ResponseSent] - {}: {}",
                    peer.to_string(),
                    request_id.to_string(),
                );
            }
        }
    }
}

impl<P: StoreParams> NetworkBehaviourEventProcess<PingEvent> for Behaviour<P> {
    fn inject_event(&mut self, event: PingEvent) {
        self.handle_ping(event)
    }
}

impl<P: StoreParams> NetworkBehaviourEventProcess<IdentifyEvent> for Behaviour<P> {
    fn inject_event(&mut self, event: IdentifyEvent) {
        self.handle_identify(event)
    }
}

impl<P: StoreParams> NetworkBehaviourEventProcess<GossipsubEvent> for Behaviour<P> {
    fn inject_event(&mut self, event: GossipsubEvent) {
        self.handle_gossipsub(event)
    }
}

impl<P: StoreParams> NetworkBehaviourEventProcess<BitswapEvent> for Behaviour<P> {
    fn inject_event(&mut self, event: BitswapEvent) {
        self.handle_bitswap(event)
    }
}

impl<P: StoreParams> NetworkBehaviourEventProcess<DiscoveryEvent> for Behaviour<P> {
    fn inject_event(&mut self, event: DiscoveryEvent) {
        self.handle_discovery(event)
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
        self.handle_request_response(event)
    }
}
