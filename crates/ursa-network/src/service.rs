//! # Ursa libp2p implementation.
//!
//! The service is bootstrapped with the following premises:
//!
//! - Load or create a new [`Keypair`] by checking the local storage.
//! - Instantiate the [`UrsaTransport`] module with quic.or(tcp) and relay support.
//! - A custom ['NetworkBehaviour'] is implemented based on [`NetworkConfig`] provided by node runner.
//! - Using the [`UrsaTransport`] and [`Behaviour`] a new [`Swarm`] is built.
//! - Two channels are created to serve (send/receive) both the network [`UrsaCommand`]'s and [`UrsaEvent`]'s.
//!
//! The [`Swarm`] events are processed in the main event loop. This loop handles dispatching [`UrsaCommand`]'s and
//! receiving [`UrsaEvent`]'s using the respective channels.

use anyhow::{anyhow, Result};

use cid::Cid;
use fnv::FnvHashMap;
use forest_ipld::Ipld;
use futures_util::stream::StreamExt;
use ipld_blockstore::BlockStore;
use libipld::DefaultParams;
use libp2p::autonat::NatStatus;
use libp2p::gossipsub::TopicHash;
use libp2p::multiaddr::Protocol;
use libp2p::swarm::{ConnectionHandler, IntoConnectionHandler, NetworkBehaviour};
use libp2p::Multiaddr;
use libp2p::{
    gossipsub::{GossipsubMessage, IdentTopic as Topic},
    identity::Keypair,
    relay::v2::client::Client as RelayClient,
    request_response::{RequestId, ResponseChannel},
    swarm::{ConnectionLimits, SwarmBuilder, SwarmEvent},
    PeerId, Swarm,
};
use libp2p_bitswap::{BitswapEvent, BitswapStore};
use rand::seq::SliceRandom;
use std::num::{NonZeroU8, NonZeroUsize};
use std::{collections::HashSet, sync::Arc};
use tracing::{debug, error, info, warn};
use ursa_index_provider::{
    advertisement::{Advertisement, MAX_ENTRIES},
    provider::{Provider, ProviderInterface},
};
use ursa_metrics::events::{track, MetricEvent};
use ursa_store::{BitswapStorage, Dag, Store};
use ursa_utils::convert_cid;

use crate::transport::build_transport;
use crate::{
    behaviour::{Behaviour, BehaviourEvent, BitswapInfo, BlockSenderChannel},
    codec::protocol::{UrsaExchangeRequest, UrsaExchangeResponse},
    config::NetworkConfig,
};
use metrics::Label;
use tokio::sync::oneshot;
use tokio::{
    select,
    sync::mpsc::{channel, Receiver, Sender},
};

pub const URSA_GLOBAL: &str = "/ursa/global";
pub const MESSAGE_PROTOCOL: &[u8] = b"/ursa/message/0.0.1";
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
pub enum UrsaCommand {
    GetBitswap {
        cid: Cid,
        query: BitswapType,
        sender: BlockSenderChannel<()>,
    },

    Put {
        cid: Cid,
        sender: oneshot::Sender<Result<()>>,
    },

    GetPeers {
        sender: oneshot::Sender<HashSet<PeerId>>,
    },

    Index {
        cids: Vec<Cid>,
        sender: oneshot::Sender<Result<Vec<Cid>>>,
    },

    SendRequest {
        peer_id: PeerId,
        request: UrsaExchangeRequest,
        channel: oneshot::Sender<Result<UrsaExchangeResponse>>,
    },

    SendResponse {
        request_id: RequestId,
        response: UrsaExchangeResponse,
        channel: oneshot::Sender<Result<()>>,
    },

    GossipsubMessage {
        topic: Topic,
        message: GossipsubMessage,
    },
}

#[derive(Debug)]
pub enum BitswapType {
    Get,
    Sync,
}

#[derive(Debug)]
pub enum UrsaEvent {
    /// An event trigger when remote peer connects.
    PeerConnected(PeerId),
    /// An event trigger when remote peer disconnects.
    PeerDisconnected(PeerId),
    BitswapEvent(BitswapEvent),
    /// A Gossip message request was received from a peer.
    GossipsubMessage(GossipsubMessage),
    /// A message request was received from a peer.
    /// Attached is a channel for returning a response.
    RequestMessage {
        request: UrsaExchangeRequest,
        channel: ResponseChannel<UrsaExchangeResponse>,
    },
}

pub struct UrsaService<S> {
    /// Store
    store: Arc<Store<S>>,
    /// index provider
    index_provider: Provider<S>,
    /// The main libp2p swarm emitting events.
    swarm: Swarm<Behaviour<DefaultParams>>,
    /// Handles outbound messages to peers
    command_sender: Sender<UrsaCommand>,
    /// Handles inbound messages from peers
    command_receiver: Receiver<UrsaCommand>,
    /// Handles events emitted by the ursa network
    event_sender: Sender<UrsaEvent>,
    /// Handles events received by the ursa network
    event_receiver: Receiver<UrsaEvent>,
    /// hashmap for keeping track of rpc response channels
    response_channels: FnvHashMap<Cid, Vec<BlockSenderChannel<()>>>,
}

impl<S> UrsaService<S>
where
    S: BlockStore + Sync + Send + 'static,
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
    ) -> Self {
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

        // subscribe to topic
        let topic = Topic::new(URSA_GLOBAL);
        if let Err(error) = swarm.behaviour_mut().subscribe(&topic) {
            warn!("Failed to subscribe with topic: {}", error);
        }

        // boostrap with kademlia
        if let Err(error) = swarm.behaviour_mut().bootstrap() {
            warn!("Failed to bootstrap with Kademlia: {}", error);
        }

        let (event_sender, event_receiver) = channel(512);
        let (command_sender, command_receiver) = channel(512);

        UrsaService {
            swarm,
            index_provider,
            store,
            command_sender,
            command_receiver,
            event_sender,
            event_receiver,
            response_channels: Default::default(),
        }
    }

    pub fn command_sender(&self) -> Sender<UrsaCommand> {
        self.command_sender.clone()
    }

    /// Handle swarm events
    pub fn handle_swarm_event(&mut self, event: SwarmEventType) -> Result<()> {
        let mut blockstore = BitswapStorage(self.store.clone());

        match event {
            SwarmEvent::Behaviour(event) => match event {
                BehaviourEvent::Bitswap(BitswapInfo {
                    cid,
                    query_id,
                    block_found,
                }) => {
                    self.swarm.behaviour_mut().cancel(query_id);
                    let labels = vec![
                        Label::new("cid", format!("{}", cid)),
                        Label::new("query_id", format!("{}", query_id)),
                        Label::new("block_found", format!("{}", block_found)),
                    ];

                    track(MetricEvent::Bitswap, Some(labels), None);

                    if let Some(chans) = self.response_channels.remove(&cid) {
                        // TODO: in some cases, the insert takes few milliseconds after query complete is received
                        // wait for block to be inserted
                        let bitswap_cid = convert_cid(cid.to_bytes());
                        if let true = block_found {
                            loop {
                                if blockstore.contains(&bitswap_cid).unwrap() {
                                    break;
                                }
                            }
                        }

                        for chan in chans.into_iter() {
                            if blockstore.contains(&bitswap_cid).unwrap() {
                                if chan.send(Ok(())).is_err() {
                                    error!("[BehaviourEvent::Bitswap] - Bitswap response channel send failed");
                                }
                            } else {
                                error!("[BehaviourEvent::Bitswap] - block not found.");
                                if chan.send(Err(anyhow!("The requested block with cid {:?} is not found with any peers", cid))).is_err() {
                                    error!("[BehaviourEvent::Bitswap] - Bitswap response channel send failed");
                                }
                            }
                        }
                    } else {
                        debug!("[BehaviourEvent::Bitswap] - Received Bitswap response, but response channel cannot be found");
                    }
                    Ok(())
                }
                BehaviourEvent::GossipMessage {
                    peer,
                    topic,
                    message,
                } => {
                    debug!("[BehaviourEvent::Gossip] - received from {:?}", peer);
                    let labels = vec![
                        Label::new("peer", format!("{}", peer)),
                        Label::new("topic", format!("{}", topic)),
                        Label::new("message", format!("{:?}", message)),
                    ];

                    track(MetricEvent::GossipMessage, Some(labels), None);

                    if self.swarm.is_connected(&peer) {
                        let event_sender = self.event_sender.clone();

                        tokio::task::spawn(async move {
                            let status = event_sender.send(UrsaEvent::GossipsubMessage(message));

                            if status.await.is_err() {
                                warn!("[BehaviourEvent::Gossip] - failed to publish message to topic: {:?}", topic);
                            }
                        });
                    }
                    Ok(())
                }
                BehaviourEvent::RequestMessage {
                    peer,
                    request,
                    channel,
                } => {
                    debug!("[BehaviourEvent::RequestMessage] {} ", peer);
                    let labels = vec![
                        Label::new("peer", format!("{}", peer)),
                        Label::new("request", format!("{:?}", request)),
                        Label::new("channel", format!("{:?}", channel)),
                    ];

                    track(MetricEvent::RequestMessage, Some(labels), None);

                    let event_sender = self.event_sender.clone();
                    tokio::task::spawn(async move {
                        if event_sender
                            .send(UrsaEvent::RequestMessage { request, channel })
                            .await
                            .is_err()
                        {
                            warn!("[BehaviourEvent::RequestMessage] - failed to send request to peer: {:?}", peer);
                        }
                    });

                    Ok(())
                }
                BehaviourEvent::PeerConnected(peer_id) => {
                    debug!(
                        "[BehaviourEvent::PeerConnected] - Peer connected {:?}",
                        peer_id
                    );

                    track(MetricEvent::PeerConnected, None, None);

                    let event_sender = self.event_sender.clone();
                    tokio::task::spawn(async move {
                        if event_sender
                            .send(UrsaEvent::PeerConnected(peer_id))
                            .await
                            .is_err()
                        {
                            warn!("[BehaviourEvent::PeerConnected] - failed to send peer connection message: {:?}", peer_id);
                        }
                    });
                    Ok(())
                }
                BehaviourEvent::PeerDisconnected(peer_id) => {
                    debug!(
                        "[BehaviourEvent::PeerDisconnected] - Peer disconnected {:?}",
                        peer_id
                    );

                    track(MetricEvent::PeerDisconnected, None, None);

                    let event_sender = self.event_sender.clone();
                    tokio::task::spawn(async move {
                        if event_sender
                            .send(UrsaEvent::PeerDisconnected(peer_id))
                            .await
                            .is_err()
                        {
                            warn!("[BehaviourEvent::PeerDisconnected] - failed to send peer disconnect message: {:?}", peer_id);
                        }
                    });
                    Ok(())
                }
                BehaviourEvent::NatStatusChanged { old, new } => {
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
                    Ok(())
                }
                BehaviourEvent::RelayReservationOpened { peer_id } => {
                    debug!("Relay reservation opened for peer {}", peer_id);
                    track(MetricEvent::RelayReservationOpened, None, None);
                    Ok(())
                }
                BehaviourEvent::RelayReservationClosed { peer_id } => {
                    debug!("Relay reservation closed for peer {}", peer_id);
                    track(MetricEvent::RelayReservationClosed, None, None);
                    Ok(())
                }
                BehaviourEvent::RelayCircuitOpened => {
                    debug!("Relay circuit opened");
                    track(MetricEvent::RelayCircuitOpened, None, None);
                    Ok(())
                }
                BehaviourEvent::RelayCircuitClosed => {
                    debug!("Relay circuit closed");
                    track(MetricEvent::RelayCircuitClosed, None, None);
                    Ok(())
                }
                BehaviourEvent::StartPublish { public_address } => {
                    let mut address = Multiaddr::empty();
                    let peer_id = *self.swarm.local_peer_id();
                    for protocol in public_address.into_iter() {
                        match protocol {
                            Protocol::Ip6(ip) => address.push(Protocol::Ip6(ip)),
                            Protocol::Ip4(ip) => address.push(Protocol::Ip4(ip)),
                            Protocol::Tcp(port) => address.push(Protocol::Tcp(port)),
                            _ => {}
                        }
                    }
                    let root_cids = self.index_provider.get_mut_root_cids();
                    let mut cid_queue = root_cids.write().unwrap();

                    while !cid_queue.is_empty() {
                        let root_cid = cid_queue.pop_front().unwrap();
                        let context_id = root_cid.to_bytes();
                        info!(
                            "Creating advertisement for cids under root cid: {:?}.",
                            root_cid
                        );

                        let addresses: Vec<String> =
                            [address.clone()].iter().map(|m| m.to_string()).collect();
                        let advertisement =
                            Advertisement::new(context_id.clone(), peer_id, addresses, false);
                        let provider_id = self.index_provider.create(advertisement).unwrap();

                        let dag = self
                            .store
                            .dag_traversal(&(convert_cid(root_cid.to_bytes())))?;
                        let entries = dag
                            .iter()
                            .map(|d| return Ipld::Bytes(d.0.hash().to_bytes()))
                            .collect::<Vec<Ipld>>();
                        let chunks: Vec<&[Ipld]> = entries.chunks(MAX_ENTRIES).collect();

                        info!("Inserting Index chunks.");
                        for chunk in chunks.iter() {
                            let entries_bytes = forest_encoding::to_vec(&chunk)?;
                            self.index_provider
                                .add_chunk(entries_bytes, provider_id)
                                .expect(" adding chunk to advertisement should not fail!");
                        }
                        info!("Publishing the advertisement now");
                        self.index_provider
                            .publish(provider_id)
                            .expect("publishing the ad should not fail");
                        if let Ok(announce_msg) = self.index_provider.create_announce_msg(peer_id) {
                            let i_topic_hash = TopicHash::from_raw("indexer/ingest/mainnet");
                            let i_topic = Topic::new("indexer/ingest/mainnet");
                            let g_msg = GossipsubMessage {
                                data: announce_msg.clone(),
                                source: None,
                                sequence_number: None,
                                topic: i_topic_hash,
                            };
                            match self.swarm.behaviour_mut().publish(i_topic, g_msg.clone()) {
                                Ok(res) => {
                                    info!("gossiping the new advertisement done : {:}", res);
                                }
                                Err(e) => {
                                    warn!("there was an error while gossiping the announcement, will try to announce via http");
                                    warn!("{:?}", e);
                                    // make an http announcement if gossiping fails
                                    let provider_copy = self.index_provider.clone();

                                    tokio::task::spawn(async move {
                                        provider_copy.announce_http_message(announce_msg).await;
                                    });
                                }
                            }
                        }
                    }

                    Ok(())
                }
            },
            // Do we need to handle any of the below events?
            SwarmEvent::Dialing { .. }
            | SwarmEvent::BannedPeer { .. }
            | SwarmEvent::NewListenAddr { .. }
            | SwarmEvent::ListenerError { .. }
            | SwarmEvent::ListenerClosed { .. }
            | SwarmEvent::ConnectionClosed { .. }
            | SwarmEvent::ExpiredListenAddr { .. }
            | SwarmEvent::IncomingConnection { .. }
            | SwarmEvent::ConnectionEstablished { .. }
            | SwarmEvent::IncomingConnectionError { .. }
            | SwarmEvent::OutgoingConnectionError { .. } => Ok(()),
        }
    }

    /// Handle commands
    pub fn handle_command(&mut self, command: UrsaCommand) -> Result<()> {
        match command {
            UrsaCommand::GetBitswap { cid, query, sender } => {
                let peers = self.swarm.behaviour_mut().peers();
                if peers.is_empty() {
                    error!(
                        "There were no peers provided and the block does not exist in local store"
                    );
                    return sender
                        .send(Err(anyhow!(
                        "There were no peers provided and the block does not exist in local store"
                    )))
                        .map_err(|_| anyhow!("Failed to get a bitswap block!"));
                } else {
                    if let Some(chans) = self.response_channels.get_mut(&cid) {
                        chans.push(sender);
                    } else {
                        self.response_channels.insert(cid, vec![sender]);
                    }
                    match query {
                        BitswapType::Get => self
                            .swarm
                            .behaviour_mut()
                            .get_block(cid, peers.iter().copied()),
                        BitswapType::Sync => self
                            .swarm
                            .behaviour_mut()
                            .sync_block(cid, peers.into_iter().collect()),
                    }
                }
                Ok(())
            }
            UrsaCommand::Put { cid, sender } => Ok(()),
            UrsaCommand::GetPeers { sender } => {
                let peers = self.swarm.behaviour_mut().peers();
                sender
                    .send(peers)
                    .map_err(|_| anyhow!("Failed to get Libp2p peers"))
            }
            UrsaCommand::Index { cids, sender } => {
                let root_cid = cids[0];
                let root_cids = self.index_provider.get_mut_root_cids();
                let mut rlock = root_cids.write().unwrap();
                rlock.push_back(root_cid);

                let swarm = self.swarm.behaviour_mut();
                let addrs = swarm.public_address();
                if let Some(public_address) = addrs {
                    swarm.publish_ad(public_address.clone())?;
                } else {
                    warn!("Public address not available. If autonat is disabled and node is private, the content will not be indexed.\
                     Otherwise the autonat will get the public address soon and node will start indexing the content");
                }

                sender
                    .send(Ok(cids))
                    .map_err(|_| anyhow!("Failed to index cid!"))
            }
            UrsaCommand::SendRequest {
                peer_id,
                request,
                channel,
            } => self
                .swarm
                .behaviour_mut()
                .send_request(peer_id, request, channel),
            UrsaCommand::SendResponse {
                request_id,
                response,
                channel,
            } => todo!(),
            UrsaCommand::GossipsubMessage { topic, message } => {
                if let Err(error) = self.swarm.behaviour_mut().publish(topic, message) {
                    warn!(
                        "[UrsaCommand::GossipsubMessage] - Failed to publish message to topic {:?} with error {:?}:",
                        URSA_GLOBAL, error
                    );
                }
                Ok(())
            }
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
mod tests {
    use super::*;

    use crate::codec::protocol::RequestType;
    use async_fs::File;
    use db::{rocks::RocksDb, rocks_config::RocksDbConfig};
    use futures::io::BufReader;
    use fvm_ipld_car::{load_car, CarReader};
    use libipld::{cbor::DagCborCodec, ipld, multihash::Code, Block, DefaultParams, Ipld};
    use simple_logger::SimpleLogger;
    use std::{str::FromStr, vec};
    use tracing::log::LevelFilter;
    use ursa_index_provider::config::ProviderConfig;
    use ursa_store::Store;

    fn create_block(ipld: Ipld) -> Block<DefaultParams> {
        Block::encode(DagCborCodec, Code::Blake3_256, &ipld).unwrap()
    }

    fn network_init(
        config: &mut NetworkConfig,
        store: Arc<Store<RocksDb>>,
        index_store: Arc<Store<RocksDb>>,
    ) -> (UrsaService<RocksDb>, PeerId) {
        let keypair = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(keypair.public());
        config.bootstrap_nodes = ["/ip4/127.0.0.1/tcp/6009"]
            .iter()
            .map(|node| node.parse().unwrap())
            .collect();

        let provider_config = ProviderConfig::default();
        let index_provider = Provider::new(keypair.clone(), index_store, provider_config.clone());

        let service = UrsaService::new(keypair, &config, Arc::clone(&store), index_provider);

        (service, local_peer_id)
    }

    fn setup_logger(level: LevelFilter) {
        SimpleLogger::new()
            .with_level(level)
            .with_utc_timestamps()
            .init()
            .unwrap()
    }

    fn get_store(path: &str) -> Arc<Store<RocksDb>> {
        let db = Arc::new(
            RocksDb::open(path, &RocksDbConfig::default()).expect("Opening RocksDB must succeed"),
        );
        Arc::new(Store::new(Arc::clone(&db)))
    }

    fn get_block(content: &[u8]) -> Block<DefaultParams> {
        create_block(ipld!(&content[..]))
    }

    fn insert_block(mut s: BitswapStorage<RocksDb>, b: &Block<DefaultParams>) {
        match s.insert(b) {
            Err(err) => error!(
                "there was an error while inserting into the blockstore {:?}",
                err
            ),
            Ok(()) => info!("block inserted successfully"),
        }
    }

    #[tokio::test]
    async fn test_network_start() {
        setup_logger(LevelFilter::Debug);

        let db = RocksDb::open("test_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");
        let provider_db = RocksDb::open("index_provider_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");

        let db = Arc::new(db);
        let store = Arc::new(Store::new(Arc::clone(&db)));
        let index_store = Arc::new(Store::new(Arc::clone(&Arc::new(provider_db))));

        let (mut service, _) = network_init(
            &mut NetworkConfig::default(),
            Arc::clone(&store),
            Arc::clone(&index_store),
        );

        loop {
            match service.swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("SwarmEvent::NewListenAddr: {:?}:", address);
                    break;
                }
                _ => {
                    panic!("test_network_start: Failed!")
                }
            }
        }
    }

    #[tokio::test]
    async fn test_network_gossip() -> Result<()> {
        setup_logger(LevelFilter::Debug);
        let mut config = NetworkConfig::default();
        let topic = Topic::new(URSA_GLOBAL);

        let db = RocksDb::open("test_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");
        let provider_db = RocksDb::open("index_provider_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");

        let db = Arc::new(db);
        let store = Arc::new(Store::new(Arc::clone(&db)));
        let index_store = Arc::new(Store::new(Arc::clone(&Arc::new(provider_db))));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6011".parse().unwrap();
        let (node_1, _) = network_init(&mut config, Arc::clone(&store), Arc::clone(&index_store));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6012".parse().unwrap();
        let (mut node_2, _) =
            network_init(&mut config, Arc::clone(&store), Arc::clone(&index_store));

        let node_1_sender = node_1.command_sender.clone();

        tokio::task::spawn(async move { node_1.start().await.unwrap() });

        let msg = UrsaCommand::GossipsubMessage {
            topic: topic.clone(),
            message: GossipsubMessage {
                source: None,
                data: vec![1],
                sequence_number: Some(1),
                topic: topic.hash(),
            },
        };

        node_1_sender.send(msg).await?;

        loop {
            select! {
                event_2 = node_2.swarm.select_next_some() => {
                    match event_2 {
                        SwarmEvent::Behaviour(behaviour) => {
                            break;
                        },
                        other => {
                            warn!("Another event receieved {:?}", other);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_network_mdns() -> Result<()> {
        setup_logger(LevelFilter::Debug);
        let mut config = NetworkConfig {
            mdns: true,
            ..Default::default()
        };

        let db = RocksDb::open("test_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");
        let provider_db = RocksDb::open("index_provider_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");

        let db = Arc::new(db);
        let store = Arc::new(Store::new(Arc::clone(&db)));
        let index_store = Arc::new(Store::new(Arc::clone(&Arc::new(provider_db))));

        let (node_1, _) = network_init(&mut config, Arc::clone(&store), Arc::clone(&index_store));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let (node_2, _) = network_init(&mut config, Arc::clone(&store), Arc::clone(&index_store));

        tokio::task::spawn(async move { node_1.start().await.unwrap() });

        let mut swarm_2 = node_2.swarm.fuse();

        loop {
            if let Some(SwarmEvent::Behaviour(BehaviourEvent::PeerConnected(peer_id))) =
                swarm_2.next().await
            {
                info!("Node 2 PeerConnected: {:?}", peer_id);
                break;
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_network_discovery() -> Result<()> {
        setup_logger(LevelFilter::Debug);
        let mut config = NetworkConfig::default();

        let db = RocksDb::open("test_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");
        let provider_db = RocksDb::open("index_provider_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");

        let db = Arc::new(db);
        let store = Arc::new(Store::new(Arc::clone(&db)));
        let index_store = Arc::new(Store::new(Arc::clone(&Arc::new(provider_db))));

        let (node_1, _) = network_init(&mut config, Arc::clone(&store), Arc::clone(&index_store));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let (mut node_2, _) =
            network_init(&mut config, Arc::clone(&store), Arc::clone(&index_store));

        tokio::task::spawn(async move { node_1.start().await.unwrap() });

        loop {
            select! {
                event = node_2.swarm.select_next_some() => {
                    match event {
                        SwarmEvent::Behaviour(behaviour) => {
                            break;
                        },
                        other => {
                            info!("Event: {:?}", other);
                            panic!("test_network_discovery: Failed!")
                        }
                    }
                }
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_network_req_res() -> Result<()> {
        setup_logger(LevelFilter::Debug);
        let mut config = NetworkConfig::default();

        let db = RocksDb::open("test_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");
        let provider_db = RocksDb::open("index_provider_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");

        let db = Arc::new(db);
        let store = Arc::new(Store::new(Arc::clone(&db)));
        let index_store = Arc::new(Store::new(Arc::clone(&Arc::new(provider_db))));

        let (node_1, _) = network_init(&mut config, Arc::clone(&store), Arc::clone(&index_store));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let (node_2, peer_2) =
            network_init(&mut config, Arc::clone(&store), Arc::clone(&index_store));

        let node_1_sender = node_1.command_sender.clone();

        tokio::task::spawn(async move { node_1.start().await.unwrap() });

        let (sender, _) = oneshot::channel();
        let request = UrsaExchangeRequest(RequestType::CarRequest("Qm".to_string()));
        let msg = UrsaCommand::SendRequest {
            peer_id: peer_2,
            request,
            channel: sender,
        };

        node_1_sender.send(msg).await?;

        let mut swarm_2 = node_2.swarm.fuse();

        loop {
            if let Some(SwarmEvent::Behaviour(BehaviourEvent::RequestMessage { request, .. })) =
                swarm_2.next().await
            {
                info!("Node 2 RequestMessage: {:?}", request);
                break;
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_bitswap_get() -> Result<()> {
        setup_logger(LevelFilter::Info);
        let mut config = NetworkConfig::default();

        let store1 = get_store("test_db1");
        let store2 = get_store("test_db2");

        let bitswap_store_1 = BitswapStorage(store1.clone());
        let mut bitswap_store_2 = BitswapStorage(store2.clone());

        let provider_db = RocksDb::open("index_provider_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");
        let index_store = Arc::new(Store::new(Arc::clone(&Arc::new(provider_db))));

        let block = get_block(&b"hello world"[..]);
        info!("inserting block into bitswap store for node 1");
        insert_block(bitswap_store_1, &block);

        let (node_1, _) = network_init(&mut config, Arc::clone(&store1), Arc::clone(&index_store));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let (node_2, _) = network_init(&mut config, Arc::clone(&store2), Arc::clone(&index_store));

        let node_2_sender = node_2.command_sender.clone();

        tokio::task::spawn(async move { node_1.start().await.unwrap() });

        tokio::task::spawn(async move { node_2.start().await.unwrap() });

        let (sender, receiver) = oneshot::channel();
        let msg = UrsaCommand::GetBitswap {
            cid: convert_cid(block.cid().to_bytes()),
            query: BitswapType::Get,
            sender,
        };

        node_2_sender.send(msg).await?;

        futures::executor::block_on(async {
            info!("waiting for msg on block receive channel...");
            let value = receiver.await.expect("Unable to receive from channel");
            if let Ok(_val) = value {
                let store_2_block = bitswap_store_2
                    .get(&convert_cid(block.cid().to_bytes()))
                    .unwrap();
                assert_eq!(store_2_block, Some(block.data().to_vec()));
            }
        });

        Ok(())
    }

    #[tokio::test]
    async fn test_bitswap_get_block_not_found() -> Result<()> {
        setup_logger(LevelFilter::Info);
        let mut config = NetworkConfig::default();

        let store1 = get_store("test_db1");
        let store2 = get_store("test_db2");
        let provider_db = RocksDb::open("index_provider_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");
        let index_store = Arc::new(Store::new(Arc::clone(&Arc::new(provider_db))));

        let (node_1, _) = network_init(&mut config, Arc::clone(&store1), Arc::clone(&index_store));

        let block = get_block(&b"hello world"[..]);

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let (node_2, _) = network_init(&mut config, Arc::clone(&store2), Arc::clone(&index_store));

        let node_2_sender = node_2.command_sender.clone();

        tokio::task::spawn(async move { node_1.start().await.unwrap() });

        tokio::task::spawn(async move { node_2.start().await.unwrap() });

        let (sender, receiver) = oneshot::channel();

        let msg = UrsaCommand::GetBitswap {
            cid: convert_cid(block.cid().to_bytes()),
            query: BitswapType::Get,
            sender,
        };

        node_2_sender.send(msg).await?;

        futures::executor::block_on(async {
            info!("waiting for msg on block receive channel...");
            let value = receiver.await.expect("Unable to receive from channel");
            // TODO: fix the assertion for this test
            match value {
                Err(val) => assert_eq!(
                    val.to_string(),
                    format!(
                        "The requested block with cid {:?} is not found with any peers",
                        *block.cid()
                    )
                ),
                _ => {}
            }
        });

        Ok(())
    }

    #[tokio::test]
    async fn add_block() -> Result<()> {
        setup_logger(LevelFilter::Info);
        let db = Arc::new(
            RocksDb::open("../test_db", &RocksDbConfig::default())
                .expect("Opening RocksDB must succeed"),
        );
        let store = Arc::new(Store::new(Arc::clone(&db)));

        let mut bitswap_store = BitswapStorage(store.clone());

        let block = get_block(&b"hello world"[..]);
        info!("inserting block into bitswap store for node");
        let cid = convert_cid(block.cid().to_bytes());
        let string_cid = Cid::to_string(&cid);
        info!("block cid to string : {:?}", string_cid);

        if let Err(err) = bitswap_store.insert(&block) {
            error!(
                "there was an error while inserting into the blockstore {:?}",
                err
            );
        } else {
            info!("block inserted successfully");
        }
        info!("{:?}", bitswap_store.contains(&convert_cid(cid.to_bytes())));

        Ok(())
    }

    #[tokio::test]
    async fn get_block_local() -> Result<()> {
        setup_logger(LevelFilter::Info);
        let db1 = Arc::new(
            RocksDb::open("test_db2", &RocksDbConfig::default())
                .expect("Opening RocksDB must succeed"),
        );

        let store1 = Arc::new(Store::new(Arc::clone(&db1)));
        let mut bitswap_store_1 = BitswapStorage(store1.clone());

        let cid =
            Cid::from_str("bafkreif2opfibjypwkjzzry3jbibcjqcjwnpoqpeiqw75eu3s3u3zbdszq").unwrap();

        if let Ok(res) = bitswap_store_1.contains(&convert_cid(cid.to_bytes())) {
            println!("block exists in current db: {:?}", res);
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_bitswap_sync() -> Result<()> {
        setup_logger(LevelFilter::Info);
        let mut config = NetworkConfig::default();

        let store1 = get_store("test_db1");
        let store2 = get_store("test_db2");

        let mut bitswap_store2 = BitswapStorage(store2.clone());
        let provider_db = RocksDb::open("index_provider_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");
        let index_store = Arc::new(Store::new(Arc::clone(&Arc::new(provider_db))));

        let path = "../car_files/text_mb.car";

        // put the car file in store 1
        // patch fix blocking io is not good
        let file = File::open(path).await?;
        let reader = BufReader::new(file);
        let cids = load_car(store1.blockstore(), reader).await?;

        let file_h = File::open(path).await?;
        let reader_h = BufReader::new(file_h);
        let mut car_reader = CarReader::new(reader_h).await?;

        let mut cids_vec = Vec::<Cid>::new();
        while let Some(block) = car_reader.next_block().await? {
            cids_vec.push(block.cid);
        }

        let (node_1, _) = network_init(&mut config, Arc::clone(&store1), Arc::clone(&index_store));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let (node_2, _) = network_init(&mut config, Arc::clone(&store2), Arc::clone(&index_store));

        let node_2_sender = node_2.command_sender.clone();

        tokio::task::spawn(async move { node_1.start().await.unwrap() });

        tokio::task::spawn(async move { node_2.start().await.unwrap() });

        let (sender, receiver) = oneshot::channel();

        let msg = UrsaCommand::GetBitswap {
            cid: cids[0],
            query: BitswapType::Sync,
            sender,
        };

        let _ = node_2_sender.send(msg);

        futures::executor::block_on(async {
            info!("waiting for msg on block receive channel...");
            let value = receiver.await.expect("Unable to receive from channel");
            if let Ok(_val) = value {
                for cid in cids_vec {
                    assert!(bitswap_store2
                        .contains(&convert_cid(cid.to_bytes()))
                        .unwrap());
                }
            }
        });
        Ok(())
    }
}
