//! # Ursa libp2p implementation.
//!
//! The service is bootstrapped with the following premises:
//!
//! - Load or create a new [`Keypair`] by checking the local storage.
//! - Instantiate the [`UrsaTransport`] module with quic.or(tcp) and relay support.
//! - A custom ['NetworkBehaviour'] is implemented based on [`NetworkConfig`] provided by node runner.
//! - Using the [`UrsaTransport`] and [`Behaviour`] a new [`Swarm`] is built.
//! - Two channels are created to serve (send/receive) both the network [`NetworkCommand`]'s and [`UrsaEvent`]'s.
//!
//! The [`Swarm`] events are processed in the main event loop. This loop handles dispatching [`NetworkCommand`]'s and
//! receiving [`UrsaEvent`]'s using the respective channels.

use anyhow::{anyhow, Error, Result};
use bytes::Bytes;
use cid::Cid;
use db::Store as Store_;
use fnv::FnvHashMap;
use forest_ipld::Ipld;
use futures_util::stream::StreamExt;
use fvm_ipld_blockstore::Blockstore;
use libipld::DefaultParams;
use libp2p::autonat::{Event as AutonatEvent, NatStatus};
use libp2p::dcutr::behaviour::Event as DcutrEvent;
use libp2p::gossipsub::error::{PublishError, SubscriptionError};
use libp2p::gossipsub::{MessageId, TopicHash};
use libp2p::identify::Event as IdentifyEvent;
use libp2p::multiaddr::Protocol;
use libp2p::ping::Event as PingEvent;
use libp2p::request_response::{RequestResponseEvent, RequestResponseMessage};
use libp2p::swarm::{ConnectionHandler, IntoConnectionHandler, NetworkBehaviour};
use libp2p::Multiaddr;
use libp2p::{
    gossipsub::IdentTopic as Topic,
    identity::Keypair,
    relay::v2::client::Client as RelayClient,
    request_response::{RequestId, ResponseChannel},
    swarm::{ConnectionLimits, SwarmBuilder, SwarmEvent},
    PeerId, Swarm,
};
use libp2p_bitswap::{BitswapEvent, BitswapStore, QueryId};
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::num::{NonZeroU8, NonZeroUsize};
use std::{collections::HashSet, sync::Arc};
use tracing::{debug, error, info, trace, warn};
use ursa_index_provider::{
    advertisement::{Advertisement, MAX_ENTRIES},
    provider::{Provider, ProviderInterface},
};
use ursa_metrics::events::{track, MetricEvent};
use ursa_store::{BitswapStorage, Dag, Store};
use ursa_utils::convert_cid;

use crate::discovery::{DiscoveryEvent, URSA_KAD_PROTOCOL};
use crate::transport::build_transport;
use crate::{
    behaviour::{Behaviour, BehaviourEvent},
    codec::protocol::{UrsaExchangeRequest, UrsaExchangeResponse},
    config::NetworkConfig,
};
use metrics::Label;
use tokio::sync::oneshot;
use tokio::{
    select,
    sync::mpsc::{unbounded_channel, UnboundedReceiver as Receiver, UnboundedSender as Sender},
};

pub const URSA_GLOBAL: &str = "/ursa/global";
pub const MESSAGE_PROTOCOL: &[u8] = b"/ursa/message/0.0.1";

type BlockOneShotSender<T> = oneshot::Sender<Result<T, Error>>;
type SwarmEventType = SwarmEvent<
<Behaviour<DefaultParams> as NetworkBehaviour>::OutEvent,
<
    <
        <
            Behaviour<DefaultParams> as NetworkBehaviour>::ConnectionHandler as IntoConnectionHandler
        >::Handler as ConnectionHandler
    >::Error
>;

#[derive(Debug)]
pub enum GossipsubMessage {
    /// A subscribe message.
    Subscribe {
        peer_id: PeerId,
        topic: TopicHash,
        sender: oneshot::Sender<Result<bool, SubscriptionError>>,
    },
    /// A subscribe message.
    Unsubscribe {
        peer_id: PeerId,
        topic: TopicHash,
        sender: oneshot::Sender<Result<bool, PublishError>>,
    },
    /// Publish a message to a specific topic.
    Publish {
        topic: TopicHash,
        data: Bytes,
        sender: oneshot::Sender<Result<MessageId, PublishError>>,
    },
}

#[derive(Debug)]
pub enum GossipsubEvent {
    /// A message has been received.
    Message {
        /// The peer that forwarded us this message.
        peer_id: PeerId,
        /// The [`MessageId`] of the message. This should be referenced by the application when
        /// validating a message (if required).
        message_id: MessageId,
        /// The decompressed message itself.
        message: libp2p::gossipsub::GossipsubMessage,
    },
    /// A remote subscribed to a topic.
    Subscribed {
        /// Remote that has subscribed.
        peer_id: PeerId,
        /// The topic it has subscribed to.
        topic: TopicHash,
    },
    /// A remote unsubscribed from a topic.
    Unsubscribed {
        /// Remote that has unsubscribed.
        peer_id: PeerId,
        /// The topic it has subscribed from.
        topic: TopicHash,
    },
}

/// [network]'s events
/// Requests and failure events emitted by the `NetworkBehaviour`.
#[derive(Debug)]
pub enum NetworkEvent {
    /// An event trigger when remote peer connects.
    PeerConnected(PeerId),
    /// An event trigger when remote peer disconnects.
    PeerDisconnected(PeerId),
    /// A Gossip message request was received from a peer.
    Gossipsub(GossipsubEvent),
    /// A message request was received from a peer.
    RequestMessage { request_id: RequestId },
    /// A bitswap HAVE event generated by the service.
    BitswapHave { cid: Cid, query_id: QueryId },
    /// A bitswap WANT event generated by the service.
    BitswapWant { cid: Cid, query_id: QueryId },
}

#[derive(Debug)]
pub enum NetworkCommand {
    GetBitswap {
        cid: Cid,
        sender: BlockOneShotSender<()>,
    },

    Put {
        cid: Cid,
        sender: oneshot::Sender<Result<()>>,
    },

    GetPeers {
        sender: oneshot::Sender<HashSet<PeerId>>,
    },

    SendRequest {
        peer_id: PeerId,
        request: UrsaExchangeRequest,
        channel: oneshot::Sender<Result<UrsaExchangeResponse>>,
    },

    GossipsubMessage {
        peer_id: PeerId,
        message: GossipsubMessage,
    },
}

pub struct UrsaService<S> {
    /// Store.
    store: Arc<Store<S>>,
    /// index provider.
    index_provider: Provider<S>,
    /// The main libp2p swarm emitting events.
    swarm: Swarm<Behaviour<DefaultParams>>,
    /// Handles outbound messages to peers.
    command_sender: Sender<NetworkCommand>,
    /// Handles inbound messages from peers.
    command_receiver: Receiver<NetworkCommand>,
    /// Handles events emitted by the ursa network.
    event_sender: Sender<NetworkEvent>,
    /// Handles events received by the ursa network.
    event_receiver: Receiver<NetworkEvent>,
    /// Bitswap pending queries.
    bitswap_queries: FnvHashMap<QueryId, Cid>,
    /// hashmap for keeping track of rpc response channels.
    response_channels: FnvHashMap<Cid, Vec<BlockOneShotSender<()>>>,
    /// Pending requests.
    pending_requests: HashMap<RequestId, ResponseChannel<UrsaExchangeResponse>>,
    /// Pending responses.
    pending_responses: HashMap<RequestId, oneshot::Sender<Result<UrsaExchangeResponse>>>,
}

impl<S> UrsaService<S>
where
    S: Blockstore + Store_ + Send + Sync + 'static,
{
    /// Init a new [`UrsaService`] based on [`NetworkConfig`]
    ///
    /// For ursa `keypair` we use ed25519 either
    /// checking for a local store or creating a new keypair.
    ///
    /// For ursa `transport` we build a default QUIC layer and
    /// fail over to tcp.
    ///
    /// For ursa behaviour we use [`Behaviour`].
    ///
    /// We construct a [`Swarm`] with [`UrsaTransport`] and [`Behaviour`]
    /// listening on [`NetworkConfig`] `swarm_addr`.
    ///
    pub fn new(
        keypair: Keypair,
        config: &NetworkConfig,
        store: Arc<Store<S>>,
        index_provider: Provider<S>,
    ) -> Result<Self> {
        let local_peer_id = PeerId::from(keypair.public());

        let (relay_transport, relay_client) = if config.relay_client {
            if !config.autonat {
                error!("Relay client requires autonat to know if we are behind a NAT");
            }

            let (relay_transport, relay_behavior) =
                RelayClient::new_transport_and_behaviour(keypair.public().into());
            (Some(relay_transport), Some(relay_behavior))
        } else {
            (None, None)
        };

        let transport = build_transport(&keypair, config, relay_transport);

        let bitswap_store = BitswapStorage(store.clone());

        let behaviour = Behaviour::new(&keypair, config, bitswap_store, relay_client);

        let limits = ConnectionLimits::default()
            .with_max_pending_incoming(Some(2 << 9))
            .with_max_pending_outgoing(Some(2 << 9))
            .with_max_established_incoming(Some(2 << 9))
            .with_max_established_outgoing(Some(2 << 9))
            .with_max_established_per_peer(Some(8));

        let mut swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id)
            .notify_handler_buffer_size(NonZeroUsize::new(2 << 7).unwrap())
            .connection_event_buffer_size(2 << 7)
            .dial_concurrency_factor(NonZeroU8::new(8).unwrap())
            .connection_limits(limits)
            .build();

        for to_dial in &config.bootstrap_nodes {
            swarm
                .dial(to_dial.clone())
                .map_err(|err| anyhow!("{}", err))
                .unwrap();
        }

        for addr in &config.swarm_addrs {
            Swarm::listen_on(&mut swarm, addr.clone())
                .map_err(|err| anyhow!("{}", err))
                .unwrap();
        }

        // subscribe to topic
        let topic = Topic::new(URSA_GLOBAL);
        if let Err(error) = swarm.behaviour_mut().subscribe(&topic) {
            warn!("Failed to subscribe with topic: {}", error);
        }

        let (event_sender, event_receiver) = unbounded_channel();
        let (command_sender, command_receiver) = unbounded_channel();

        Ok(UrsaService {
            swarm,
            index_provider,
            store,
            command_sender,
            command_receiver,
            event_sender,
            event_receiver,
            response_channels: Default::default(),
            bitswap_queries: Default::default(),
            pending_requests: HashMap::default(),
            pending_responses: HashMap::default(),
        })
    }

    pub fn command_sender(&self) -> Sender<NetworkCommand> {
        self.command_sender.clone()
    }

    fn emit_event(&mut self, event: NetworkEvent) {
        let sender = self.event_sender.clone();
        tokio::task::spawn(async move {
            if let Err(error) = sender.send(event) {
                warn!("[emit_event] - failed to emit network event: {:?}.", error);
            };
        });
    }

    fn handle_ping(&mut self, ping_event: PingEvent) -> Result<()> {
        match ping_event.result {
            Ok(libp2p::ping::Success::Ping { rtt }) => {
                trace!(
                    "[PingSuccess::Ping] - with rtt {} from {} in ms",
                    rtt.as_millis(),
                    ping_event.peer.to_base58(),
                );
            }
            Ok(libp2p::ping::Success::Pong) => {
                trace!(
                    "PingSuccess::Pong] - received a ping and sent back a pong to {}",
                    ping_event.peer.to_base58()
                );
            }
            Err(libp2p::ping::Failure::Other { error }) => {
                debug!(
                    "[PingFailure::Other] - the ping failed with {} for reasons {}",
                    ping_event.peer.to_base58(),
                    error
                );
            }
            Err(libp2p::ping::Failure::Timeout) => {
                warn!(
                    "[PingFailure::Timeout] - no response was received from {}",
                    ping_event.peer.to_base58()
                );
            }
            Err(libp2p::ping::Failure::Unsupported) => {
                debug!(
                    "[PingFailure::Unsupported] - the peer {} does not support the ping protocol",
                    ping_event.peer.to_base58()
                );
            }
        }
        Ok(())
    }

    fn handle_identify(&mut self, identify_event: IdentifyEvent) -> Result<(), anyhow::Error> {
        match identify_event {
            IdentifyEvent::Received { peer_id, info } => {
                trace!(
                    "[IdentifyEvent::Received] - with version {} has been received from a peer {}.",
                    info.protocol_version,
                    peer_id
                );

                if self.swarm.behaviour().peers().contains(&peer_id) {
                    trace!(
                        "[IdentifyEvent::Received] - peer {} already known!",
                        peer_id
                    );
                }

                // check if received identify is from a peer on the same network
                // if info
                //     .protocols
                //     .iter()
                //     .any(|name| name.as_bytes() == URSA_KAD_PROTOCOL)
                // {

                // }
                self.swarm
                    .behaviour_mut()
                    .gossipsub
                    .add_explicit_peer(&peer_id);

                for address in info.listen_addrs {
                    self.swarm
                        .behaviour_mut()
                        .bitswap
                        .add_address(&peer_id, address.clone());
                    self.swarm
                        .behaviour_mut()
                        .discovery
                        .add_address(&peer_id, address.clone());
                    self.swarm
                        .behaviour_mut()
                        .request_response
                        .add_address(&peer_id, address.clone());
                }
            }
            IdentifyEvent::Sent { .. }
            | IdentifyEvent::Pushed { .. }
            | IdentifyEvent::Error { .. } => {}
        }
        Ok(())
    }

    fn handle_autonat(&mut self, autonat_event: AutonatEvent) -> Result<(), anyhow::Error> {
        match autonat_event {
            AutonatEvent::StatusChanged { old, new } => {
                match (old, new) {
                    (NatStatus::Unknown, NatStatus::Private) => {
                        let behaviour = self.swarm.behaviour_mut();
                        if behaviour.is_relay_client_enabled() {
                            // get random bootstrap node and listen on their relay
                            if let Some((relay_peer, relay_addr)) = behaviour
                                .discovery()
                                .bootstrap_addrs()
                                .choose(&mut rand::thread_rng())
                            {
                                let addr = relay_addr
                                    .clone()
                                    .with(Protocol::P2p((*relay_peer).into()))
                                    .with(Protocol::P2pCircuit);
                                warn!("Private NAT detected. Establishing public relay address on peer {}", addr);
                                self.swarm
                                    .listen_on(addr)
                                    .expect("failed to listen on relay");
                            }
                        }
                    }
                    (_, NatStatus::Public(addr)) => {
                        info!("Public Nat verified! Public listening address: {}", addr);
                    }
                    (old, new) => {
                        warn!("NAT status changed from {:?} to {:?}", old, new);
                    }
                }
            }
            AutonatEvent::InboundProbe(_) | AutonatEvent::OutboundProbe(_) => (),
        }
        Ok(())
    }

    fn handle_dcutr(&self, dcutr_event: DcutrEvent) -> Result<(), anyhow::Error> {
        //     debug!("Relay circuit closed");
        //     track(MetricEvent::RelayCircuitClosed, None, None);
        Ok(())
    }

    fn handle_bitswap(
        &mut self,
        bitswap_event: libp2p_bitswap::BitswapEvent,
    ) -> Result<(), anyhow::Error> {
        match bitswap_event {
            libp2p_bitswap::BitswapEvent::Progress(query_id, _) => {
                debug!("bitswap request in progress with, id: {}", query_id);
            }
            libp2p_bitswap::BitswapEvent::Complete(query_id, result) => match result {
                Ok(_) => match self.bitswap_queries.remove(&query_id) {
                    Some(cid) => {
                        let labels = vec![
                            Label::new("cid", format!("{}", cid)),
                            Label::new("query_id", format!("{}", query_id)),
                        ];

                        track(MetricEvent::Bitswap, Some(labels), None);
                        self.emit_event(NetworkEvent::BitswapHave { cid, query_id });
                    }
                    _ => {
                        error!(
                            "[BitswapEvent::Complete] - Query Id {:?} not found in the hash map",
                            query_id
                        )
                    }
                },
                Err(_) => todo!(),
            },
        }
        Ok(())
    }

    fn handle_gossip(
        &mut self,
        gossip_event: libp2p::gossipsub::GossipsubEvent,
    ) -> Result<(), anyhow::Error> {
        match gossip_event {
            libp2p::gossipsub::GossipsubEvent::Message {
                propagation_source,
                message_id,
                message,
            } => {
                self.emit_event(NetworkEvent::Gossipsub(GossipsubEvent::Message {
                    peer_id: propagation_source,
                    message_id,
                    message,
                }));
            }
            libp2p::gossipsub::GossipsubEvent::Subscribed { peer_id, topic } => {
                self.emit_event(NetworkEvent::Gossipsub(GossipsubEvent::Subscribed {
                    peer_id,
                    topic,
                }));
            }
            libp2p::gossipsub::GossipsubEvent::Unsubscribed { peer_id, topic } => {
                self.emit_event(NetworkEvent::Gossipsub(GossipsubEvent::Unsubscribed {
                    peer_id,
                    topic,
                }));
            }
            libp2p::gossipsub::GossipsubEvent::GossipsubNotSupported { .. } => (),
        }
        Ok(())
    }

    fn handle_discovery(&mut self, discovery_event: DiscoveryEvent) -> Result<(), anyhow::Error> {
        match discovery_event {
            DiscoveryEvent::Connected(peer_id) => {
                debug!(
                    "[BehaviourEvent::PeerConnected] - Peer connected {:?}",
                    peer_id
                );

                track(MetricEvent::PeerConnected, None, None);
                self.emit_event(NetworkEvent::PeerConnected(peer_id));
            }
            DiscoveryEvent::Disconnected(peer_id) => {
                debug!(
                    "[BehaviourEvent::PeerDisconnected] - Peer disconnected {:?}",
                    peer_id
                );

                track(MetricEvent::PeerDisconnected, None, None);
                self.emit_event(NetworkEvent::PeerDisconnected(peer_id));
            }
        }
        Ok(())
    }

    fn handle_req_res(
        &mut self,
        req_res_event: RequestResponseEvent<UrsaExchangeRequest, UrsaExchangeResponse>,
    ) -> Result<(), anyhow::Error> {
        match req_res_event {
            RequestResponseEvent::Message { peer, message } => {
                match message {
                    RequestResponseMessage::Request {
                        request_id,
                        request,
                        channel,
                    } => {
                        trace!("[BehaviourEvent::RequestMessage] {} ", peer);
                        let labels = vec![
                            Label::new("peer", format!("{}", peer)),
                            Label::new("request", format!("{:?}", request)),
                            Label::new("channel", format!("{:?}", channel)),
                        ];

                        track(MetricEvent::RequestMessage, Some(labels), None);
                        self.emit_event(NetworkEvent::RequestMessage { request_id });
                    }
                    RequestResponseMessage::Response {
                        request_id,
                        response,
                    } => {
                        trace!(
                            "[RequestResponseMessage::Response] - {} {}: {:?}",
                            request_id,
                            peer,
                            response
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
            RequestResponseEvent::OutboundFailure { .. }
            | RequestResponseEvent::InboundFailure { .. }
            | RequestResponseEvent::ResponseSent { .. } => (),
        }
        Ok(())
    }

    /// Handle swarm events
    pub fn handle_swarm_event(&mut self, event: SwarmEventType) -> Result<()> {
        match event {
            SwarmEvent::Behaviour(event) => match event {
                BehaviourEvent::Ping(ping_event) => self.handle_ping(ping_event),
                BehaviourEvent::Identify(identify_event) => self.handle_identify(identify_event),
                BehaviourEvent::Autonat(autonat_event) => self.handle_autonat(autonat_event),
                BehaviourEvent::RelayClient(_) => Ok(()),
                BehaviourEvent::RelayServer(_) => Ok(()),
                BehaviourEvent::Dcutr(dcutr_event) => self.handle_dcutr(dcutr_event),
                BehaviourEvent::Bitswap(bitswap_event) => self.handle_bitswap(bitswap_event),
                BehaviourEvent::Gossipsub(gossip_event) => self.handle_gossip(gossip_event),
                BehaviourEvent::Discovery(discovery_event) => {
                    self.handle_discovery(discovery_event)
                }
                BehaviourEvent::RequestResponse(req_res_event) => {
                    self.handle_req_res(req_res_event)
                }
            },
            _ => {
                debug!("Unhandled swarm event {:?}", event);
                Ok(())
            }
        }
    }

    /// Handle commands
    pub fn handle_command(&mut self, command: NetworkCommand) -> Result<()> {
        match command {
            NetworkCommand::GetBitswap { cid, .. } => {
                let peers = self.swarm.behaviour_mut().peers();
                let query = self
                    .swarm
                    .behaviour_mut()
                    .get_block(cid, peers.iter().copied());

                match query {
                    Ok(query_id) => {
                        self.bitswap_queries.insert(query_id, cid);
                        self.emit_event(NetworkEvent::BitswapWant { cid, query_id });
                    }
                    _ => {
                        error!(
                            "[NetworkCommand::BitswapWant] - no block found for cid {:?}.",
                            cid
                        )
                    }
                }
            }
            NetworkCommand::Put { cid, sender } => (),
            NetworkCommand::GetPeers { sender } => {
                let peers = self.swarm.behaviour_mut().peers();
                sender
                    .send(peers)
                    .map_err(|_| anyhow!("Failed to get Libp2p peers!"))?;
            }
            NetworkCommand::SendRequest {
                peer_id,
                request,
                channel,
            } => {
                let request_id = self
                    .swarm
                    .behaviour_mut()
                    .request_response
                    .send_request(&peer_id, request);
                self.pending_responses.insert(request_id, channel);

                let event_sender = self.event_sender.clone();
                if let Err(e) = event_sender.send(NetworkEvent::RequestMessage { request_id }) {
                    warn!("Failed to send request!: {:?}", e);
                }
            }
            NetworkCommand::GossipsubMessage {
                peer_id: _,
                message,
            } => match message {
                GossipsubMessage::Subscribe {
                    peer_id: _,
                    topic,
                    sender,
                } => {
                    let subscribe = self
                        .swarm
                        .behaviour_mut()
                        .gossipsub
                        .subscribe(&Topic::new(topic.into_string()));

                    sender
                        .send(subscribe)
                        .map_err(|_| anyhow!("Failed to subscribe!"))?;
                }
                GossipsubMessage::Unsubscribe {
                    peer_id: _,
                    topic,
                    sender,
                } => {
                    let unsubscribe = self
                        .swarm
                        .behaviour_mut()
                        .gossipsub
                        .unsubscribe(&Topic::new(topic.into_string()));

                    sender
                        .send(unsubscribe)
                        .map_err(|_| anyhow!("Failed to unsubscribe!"))?;
                }
                GossipsubMessage::Publish {
                    topic,
                    data,
                    sender,
                } => {
                    let publish = self
                        .swarm
                        .behaviour_mut()
                        .publish(Topic::new(topic.into_string()), data.to_vec());

                    if let Err(e) = &publish {
                        warn!("Publish error: {e:?}");
                    }

                    sender
                        .send(publish)
                        .map_err(|_| anyhow!("Failed to publish message!"))?;
                }
            },
        }
        Ok(())
    }

    /// Dial remote peer `peer_id` at `address`
    pub fn dial(
        &mut self,
        peer_id: PeerId,
        address: Multiaddr,
        response: oneshot::Sender<Result<()>>,
    ) -> Result<()> {
        log::trace!("dial peer ({peer_id}) at address {address}");

        match self.swarm.dial(address.clone()) {
            Ok(_) => {
                self.swarm
                    .behaviour_mut()
                    .discovery
                    .add_address(&peer_id, address);
                response
                    .send(Ok(()))
                    .map_err(|_| anyhow!("{}", "Channel Dropped"))
            }
            Err(err) => response
                .send(Err(err.into()))
                .map_err(|_| anyhow!("{}", "DialError")),
        }
    }

    /// Start the ursa network service loop.
    ///
    /// Poll `swarm` and `command_receiver` from [`UrsaService`].
    /// - `swarm` handles the network events [Event].
    /// - `command_receiver` handles inbound commands [Command].
    pub async fn start(mut self) -> Result<()> {
        info!(
            "Node starting up with peerId {:?}",
            self.swarm.local_peer_id()
        );

        loop {
            select! {
                event = self.swarm.next() => {
                    let event = event.ok_or_else(|| anyhow!("Swarm Event invalid!"))?;
                    self.handle_swarm_event(event).expect("Handle swarm event.");
                },
                command = self.command_receiver.recv() => {
                    let command = command.ok_or_else(|| anyhow!("Command invalid!"))?;
                    self.handle_command(command).expect("Handle rpc command.");
                },
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/service_tests.rs"]
mod service_tests;