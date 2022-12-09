//! # Ursa Behaviour implementation.
//!
//! Ursa custom behaviour implements [`NetworkBehaviour`] with the following options:
//!
//! - [`Ping`] A `NetworkBehaviour` that responds to inbound pings and
//!   periodically sends outbound pings on every established connection.
//! - [`Identify`] A `NetworkBehaviour` that automatically identifies nodes periodically, returns information
//!   about them, and answers identify queries from other nodes.
//! - [`Bitswap`] A `NetworkBehaviour` that handles sending and receiving blocks.
//! - [`Gossipsub`] A `NetworkBehaviour` that handles the gossipsub protocol.
//! - [`DiscoveryBehaviour`]
//! - [`RequestResponse`] A `NetworkBehaviour` that implements a generic
//!   request/response protocol or protocol family, whereby each request is
//!   sent over a new substream on a connection.

use anyhow::{Error, Result};
use cid::Cid;
use fnv::FnvHashMap;
use libipld::store::StoreParams;
use libp2p::autonat::{Event, NatStatus};
use libp2p::core::connection::ConnectionId;
use libp2p::dcutr;
use libp2p::ping::Config as PingConfig;
use libp2p::swarm::behaviour::toggle::Toggle;
use libp2p::swarm::behaviour::FromSwarm;
use libp2p::swarm::{ConnectionHandler, DialError, IntoConnectionHandler};
use libp2p::{
    autonat::{Behaviour as Autonat, Config as AutonatConfig, Event as AutonatEvent},
    dcutr::behaviour::Event as DcutrEvent,
    gossipsub::{
        error::{PublishError, SubscriptionError},
        Gossipsub, GossipsubEvent, GossipsubMessage, IdentTopic as Topic, MessageId,
        PeerScoreParams, PeerScoreThresholds, TopicHash,
    },
    identify::{Behaviour as Identify, Config as IdentifyConfig, Event as IdentifyEvent},
    identity::Keypair,
    kad,
    ping::{Behaviour as Ping, Event as PingEvent, Failure as PingFailure, Success as PingSuccess},
    relay::v2::{
        client::{Client as RelayClient, Event as RelayClientEvent},
        relay::{Config as RelayConfig, Event as RelayServerEvent, Relay as RelayServer},
    },
    request_response::{
        ProtocolSupport, RequestId, RequestResponse, RequestResponseConfig, RequestResponseEvent,
        RequestResponseMessage, ResponseChannel,
    },
    swarm::{NetworkBehaviour, NetworkBehaviourAction, PollParameters},
    Multiaddr, PeerId,
};
use libp2p_bitswap::{Bitswap, BitswapConfig, BitswapEvent, BitswapStore, QueryId};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    iter,
    task::{Context, Poll},
    time::Duration,
};
use tokio::sync::oneshot;
use tracing::{debug, error, trace, warn};
use ursa_metrics::Recorder;
use ursa_utils::convert_cid;

use crate::discovery::URSA_KAD_PROTOCOL;
use crate::{
    codec::protocol::{UrsaExchangeCodec, UrsaExchangeRequest, UrsaExchangeResponse, UrsaProtocol},
    config::NetworkConfig,
    discovery::{DiscoveryBehaviour, DiscoveryEvent},
    gossipsub::UrsaGossipsub,
};

pub type BlockSenderChannel<T> = oneshot::Sender<Result<T, Error>>;

#[derive(Debug)]
pub struct BitswapInfo {
    pub cid: Cid,
    pub query_id: QueryId,
    pub block_found: bool,
}

pub const IPFS_PROTOCOL: &str = "ipfs/0.1.0";

fn ursa_agent() -> String {
    format!("ursa/{}", env!("CARGO_PKG_VERSION"))
}

/// [Behaviour]'s events
/// Requests and failure events emitted by the `NetworkBehaviour`.
#[derive(Debug)]
pub enum BehaviourEvent {
    NatStatusChanged {
        old: NatStatus,
        new: NatStatus,
    },
    /// An event trigger when remote peer connects.
    PeerConnected(PeerId),
    /// An event trigger when remote peer disconnects.
    PeerDisconnected(PeerId),
    /// A Gossip message request was received from a peer.
    Bitswap(BitswapInfo),
    GossipMessage {
        peer: PeerId,
        topic: TopicHash,
        message: GossipsubMessage,
    },
    /// A message request was received from a peer.
    /// Attached is a channel for returning a response.
    RequestMessage {
        peer: PeerId,
        request: UrsaExchangeRequest,
        channel: ResponseChannel<UrsaExchangeResponse>,
    },
    StartPublish {
        public_address: Multiaddr,
    },
}

/// Composes protocols for the behaviour of the node in the network.
#[derive(NetworkBehaviour)]
pub struct InternalBehaviour<P: StoreParams> {
    /// Alive checks.
    ping: Ping,

    /// Identify and exchange info with other peers.
    identify: Identify,

    /// autonat
    autonat: Toggle<Autonat>,

    /// Relay client. Used to listen on a relay for incoming connections.
    relay_client: Toggle<RelayClient>,

    /// Relay server. Used to allow other peers to route through the node
    relay_server: Toggle<RelayServer>,

    /// DCUtR
    dcutr: Toggle<dcutr::behaviour::Behaviour>,

    /// Bitswap for exchanging data between blocks between peers.
    bitswap: Bitswap<P>,

    /// Ursa's gossiping protocol for message propagation.
    gossipsub: Gossipsub,

    /// Kademlia discovery and bootstrap.
    discovery: DiscoveryBehaviour,

    /// request/response protocol implementation for [`UrsaProtocol`]
    request_response: RequestResponse<UrsaExchangeCodec>,
}

impl<P: StoreParams> InternalBehaviour<P> {
    pub fn new<S: BitswapStore<Params = P>>(
        keypair: &Keypair,
        config: &NetworkConfig,
        bitswap_store: S,
        relay_client: Option<libp2p::relay::v2::client::Client>,
    ) -> Self {
        let local_public_key = keypair.public();
        let local_peer_id = PeerId::from(local_public_key.clone());

        // Setup the ping behaviour
        let ping = Ping::new(PingConfig::new().with_keep_alive(true));

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
        bitswap
            .register_metrics(&Default::default())
            .expect("bitswap metrics");

        // Setup the identify behaviour
        let identify = Identify::new(
            IdentifyConfig::new(IPFS_PROTOCOL.into(), keypair.public())
                .with_agent_version(ursa_agent()),
        );

        let request_response = {
            let mut cfg = RequestResponseConfig::default();

            // todo(botch): calculate an upper limit to allow for large files
            cfg.set_request_timeout(Duration::from_secs(60));

            let protocols = iter::once((UrsaProtocol, ProtocolSupport::Full));

            RequestResponse::new(UrsaExchangeCodec, protocols, cfg)
        };

        let autonat = config
            .autonat
            .then(|| {
                let config = AutonatConfig {
                    throttle_server_period: Duration::from_secs(30),
                    ..AutonatConfig::default()
                };

                Autonat::new(local_peer_id, config)
            })
            .into();

        let relay_server = config
            .relay_server
            .then(|| RelayServer::new(local_public_key.into(), RelayConfig::default()))
            .into();

        let dcutr = config
            .relay_client
            .then(|| {
                if relay_client.is_none() {
                    panic!("relay client not instantiated");
                }
                dcutr::behaviour::Behaviour::new()
            })
            .into();

        InternalBehaviour {
            ping,
            autonat,
            relay_server,
            relay_client: relay_client.into(),
            dcutr,
            bitswap,
            identify,
            gossipsub,
            discovery,
            request_response,
        }
    }

    pub fn publish(
        &mut self,
        topic: Topic,
        data: GossipsubMessage,
    ) -> Result<MessageId, PublishError> {
        self.gossipsub.publish(topic, data.data)
    }

    pub fn public_address(&self) -> Option<&Multiaddr> {
        self.autonat.as_ref().and_then(|a| a.public_address())
    }

    pub fn peers(&self) -> HashSet<PeerId> {
        self.discovery.peers().clone()
    }

    pub fn is_relay_client_enabled(&self) -> bool {
        self.relay_client.is_enabled()
    }

    pub fn discovery(&mut self) -> &mut DiscoveryBehaviour {
        &mut self.discovery
    }

    pub fn bootstrap(&mut self) -> Result<kad::QueryId, Error> {
        self.discovery.bootstrap()
    }

    pub fn subscribe(&mut self, topic: &Topic) -> Result<bool, SubscriptionError> {
        self.gossipsub.subscribe(topic)
    }

    pub fn unsubscribe(&mut self, topic: &Topic) -> Result<bool, PublishError> {
        self.gossipsub.unsubscribe(topic)
    }
}

/// A `Networkbehaviour` that handles Ursa's different protocol implementations.
///
/// The poll function must have the same signature as the NetworkBehaviour
/// function and will be called last within the generated NetworkBehaviour implementation.
///
/// The events generated [`BehaviourEvent`].
pub struct Behaviour<P: StoreParams> {
    inner: InternalBehaviour<P>,

    /// Ursa's emitted events.
    events: VecDeque<BehaviourEvent>,

    /// Pending requests
    pending_requests: HashMap<RequestId, ResponseChannel<UrsaExchangeResponse>>,

    /// Pending responses
    pending_responses: HashMap<RequestId, oneshot::Sender<Result<UrsaExchangeResponse>>>,

    queries: FnvHashMap<QueryId, BitswapInfo>,
}

impl<P: StoreParams> Behaviour<P> {
    pub fn new<S: BitswapStore<Params = P>>(
        keypair: &Keypair,
        config: &NetworkConfig,
        bitswap_store: S,
        relay_client: Option<libp2p::relay::v2::client::Client>,
    ) -> Self {
        Self {
            inner: InternalBehaviour::new(keypair, config, bitswap_store, relay_client),
            events: VecDeque::new(),
            pending_requests: HashMap::default(),
            pending_responses: HashMap::default(),
            queries: Default::default(),
        }
    }

    pub fn publish(
        &mut self,
        topic: Topic,
        data: GossipsubMessage,
    ) -> Result<MessageId, PublishError> {
        self.inner.publish(topic, data)
    }

    pub fn public_address(&self) -> Option<&Multiaddr> {
        self.inner.public_address()
    }

    pub fn peers(&self) -> HashSet<PeerId> {
        self.inner.peers()
    }

    pub fn is_relay_client_enabled(&self) -> bool {
        self.inner.is_relay_client_enabled()
    }

    pub fn discovery(&mut self) -> &mut DiscoveryBehaviour {
        self.inner.discovery()
    }

    pub fn bootstrap(&mut self) -> Result<kad::QueryId, Error> {
        self.inner.bootstrap()
    }

    pub fn subscribe(&mut self, topic: &Topic) -> Result<bool, SubscriptionError> {
        self.inner.subscribe(topic)
    }

    pub fn unsubscribe(&mut self, topic: &Topic) -> Result<bool, PublishError> {
        self.inner.unsubscribe(topic)
    }

    pub fn publish_ad(&mut self, public_address: Multiaddr) -> Result<()> {
        self.events
            .push_back(BehaviourEvent::StartPublish { public_address });
        Ok(())
    }

    pub fn send_request(
        &mut self,
        peer: PeerId,
        request: UrsaExchangeRequest,
        sender: oneshot::Sender<Result<UrsaExchangeResponse>>,
    ) -> Result<()> {
        let request_id = self.inner.request_response.send_request(&peer, request);
        self.pending_responses.insert(request_id, sender);

        Ok(())
    }

    pub fn get_block(&mut self, cid: Cid, providers: impl Iterator<Item = PeerId>) {
        debug!("get block via rpc called, the requested cid is: {:?}", cid);
        let id = self
            .inner
            .bitswap
            .get(convert_cid(cid.to_bytes()), providers);

        self.queries.insert(
            id,
            BitswapInfo {
                query_id: id,
                cid,
                block_found: false,
            },
        );
    }

    pub fn sync_block(&mut self, cid: Cid, providers: Vec<PeerId>) {
        debug!(
            "sync block via http called, the requested root cid is: {:?}",
            cid
        );
        let c_cid = convert_cid(cid.to_bytes());
        let id = self
            .inner
            .bitswap
            .sync(c_cid, providers, std::iter::once(c_cid));
        self.queries.insert(
            id,
            BitswapInfo {
                query_id: id,
                cid,
                block_found: false,
            },
        );
    }

    pub fn cancel(&mut self, id: QueryId) {
        self.queries.remove(&id);
        self.inner.bitswap.cancel(id);
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
        debug!("[IdentifyEvent] {:?}", event);
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
                }

                // check if received identify is from a peer on the same network
                if info
                    .protocols
                    .iter()
                    .any(|name| name.as_bytes() == URSA_KAD_PROTOCOL)
                {
                    self.inner.gossipsub.add_explicit_peer(&peer_id);

                    for address in info.listen_addrs {
                        self.inner.discovery.add_address(&peer_id, address.clone());
                        self.inner
                            .request_response
                            .add_address(&peer_id, address.clone());
                    }
                }
            }
            IdentifyEvent::Sent { .. }
            | IdentifyEvent::Pushed { .. }
            | IdentifyEvent::Error { .. } => {}
        }
    }

    fn handle_autonat(&mut self, event: AutonatEvent) -> Option<BehaviourEvent> {
        debug!("[AutonatEvent] {:?}", event);
        match event {
            AutonatEvent::StatusChanged { old, new } => {
                Some(BehaviourEvent::NatStatusChanged { old, new })
            }
            Event::OutboundProbe(_) | Event::InboundProbe(_) => None,
        }
    }

    fn handle_relay_server(&mut self, event: RelayServerEvent) {
        debug!("[RelayServerEvent] {:?}", event);
    }

    fn handle_relay_client(&mut self, event: RelayClientEvent) {
        debug!("[RelayClientEvent] {:?}", event);
    }

    fn handle_dcutr(&mut self, event: DcutrEvent) {
        debug!("[DcutrEvent] {:?}", event);
    }

    fn handle_bitswap(&mut self, event: BitswapEvent) -> Option<BehaviourEvent> {
        match event {
            BitswapEvent::Progress(id, missing) => {
                debug!(
                    "progress in bitswap sync query, id: {}, missing: {}",
                    id, missing
                );
            }
            BitswapEvent::Complete(id, result) => {
                debug!(
                    "[BitswapEvent::Complete] - Bitswap Event complete for query id: {:?}",
                    id
                );
                match self.queries.remove(&id) {
                    Some(mut info) => {
                        match result {
                            Err(err) => error!("{:?}", err),
                            Ok(_res) => info.block_found = true,
                        }
                        return Some(BehaviourEvent::Bitswap(info));
                    }
                    _ => {
                        error!(
                            "[BitswapEvent::Complete] - Query Id {:?} not found in the hash map",
                            id
                        )
                    }
                }
            }
        }
        None
    }

    fn handle_gossipsub(&mut self, event: GossipsubEvent) -> Option<BehaviourEvent> {
        match event {
            GossipsubEvent::Message {
                propagation_source,
                message,
                ..
            } => {
                return Some(BehaviourEvent::GossipMessage {
                    peer: propagation_source,
                    topic: message.topic.clone(),
                    message,
                });
            }
            GossipsubEvent::Subscribed { .. } => {
                // A remote subscribed to a topic.
                // subscribe to new topic.
            }
            GossipsubEvent::Unsubscribed { .. } => {
                // A remote unsubscribed from a topic.
                // remove subscription.
            }
            GossipsubEvent::GossipsubNotSupported { .. } => {
                // A peer that does not support gossipsub has connected.
                // the scoring/rating should happen here.
                // disconnect.
            }
        }
        None
    }

    fn handle_discovery(&mut self, event: DiscoveryEvent) -> Option<BehaviourEvent> {
        match event {
            DiscoveryEvent::Connected(peer_id) => Some(BehaviourEvent::PeerConnected(peer_id)),
            DiscoveryEvent::Disconnected(peer_id) => {
                Some(BehaviourEvent::PeerDisconnected(peer_id))
            }
        }
    }

    fn handle_request_response(
        &mut self,
        event: RequestResponseEvent<UrsaExchangeRequest, UrsaExchangeResponse>,
    ) -> Option<BehaviourEvent> {
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

                        return Some(BehaviourEvent::RequestMessage {
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
        None
    }
}


impl<P: StoreParams> NetworkBehaviour for Behaviour<P> {
    type ConnectionHandler = <InternalBehaviour<P> as NetworkBehaviour>::ConnectionHandler;
    type OutEvent = BehaviourEvent;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        self.inner.new_handler()
    }

    fn addresses_of_peer(&mut self, peer: &PeerId) -> Vec<Multiaddr> {
        self.inner.addresses_of_peer(peer)
    }

    fn on_swarm_event(&mut self, event: FromSwarm<Self::ConnectionHandler>) {
        self.inner.on_swarm_event(event);
    }

    fn on_connection_handler_event(
        &mut self,
        peer_id: PeerId,
        connection_id: ConnectionId,
        event: <<Self::ConnectionHandler as IntoConnectionHandler>::Handler as ConnectionHandler>::OutEvent,
    ) {
        self.inner
            .on_connection_handler_event(peer_id, connection_id, event)
    }

    fn poll(
        &mut self,
        cx: &mut Context<'_>,
        params: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Self::OutEvent, Self::ConnectionHandler>> {
        loop {
            if let Some(event) = self.events.pop_front() {
                return Poll::Ready(NetworkBehaviourAction::GenerateEvent(event));
            }
            match self.inner.poll(cx, params) {
                Poll::Ready(NetworkBehaviourAction::GenerateEvent(event)) => {
                    let mut gen_event = None;
                    match event {
                        InternalBehaviourEvent::Ping(e) => {
                          e.record();
                          self.handle_ping(e)
                        },
                        InternalBehaviourEvent::Identify(e) => {
                          e.record();
                          self.handle_identify(e)
                        },
                        InternalBehaviourEvent::Autonat(e) => gen_event = self.handle_autonat(e),
                        InternalBehaviourEvent::RelayServer(e) => {
                            e.record();
                            self.handle_relay_server(e)
                        }
                        InternalBehaviourEvent::RelayClient(e) => self.handle_relay_client(e),
                        InternalBehaviourEvent::Bitswap(e) => gen_event = self.handle_bitswap(e),
                        InternalBehaviourEvent::Gossipsub(e) => {
                            e.record();
                            gen_event = self.handle_gossipsub(e)
                        }
                        InternalBehaviourEvent::Discovery(e) => {
                            gen_event = self.handle_discovery(e)
                        }
                        InternalBehaviourEvent::Dcutr(e) => self.handle_dcutr(e),
                        InternalBehaviourEvent::RequestResponse(e) => {
                            e.record();
                            gen_event = self.handle_request_response(e)
                        }
                    };

                    if let Some(event) = gen_event {
                        return Poll::Ready(NetworkBehaviourAction::GenerateEvent(event));
                    }
                }
                Poll::Ready(action) => return Poll::Ready(action.map_out(|_| unreachable!())),
                Poll::Pending => return Poll::Pending,
            };
        }
    }
}
