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

use async_std::{
    channel::{unbounded, Receiver, Sender},
    task,
};

use cid::Cid;
use fnv::FnvHashMap;
use forest_ipld::Ipld;
use futures::{channel::oneshot, select};
use futures_util::stream::StreamExt;
use ipld_blockstore::BlockStore;
use libipld::DefaultParams;
use libp2p::{
    autonat::NatStatus,
    core::multiaddr::Protocol,
    gossipsub::{GossipsubMessage, IdentTopic as Topic, TopicHash},
    identity::Keypair,
    relay::v2::client::Client as RelayClient,
    request_response::{RequestId, ResponseChannel},
    swarm::{ConnectionLimits, SwarmBuilder, SwarmEvent},
    PeerId, Swarm,
};
use libp2p_bitswap::{BitswapEvent, BitswapStore};
use rand::seq::SliceRandom;
use std::{
    collections::HashSet,
    num::{NonZeroU8, NonZeroUsize},
    str::FromStr,
    sync::Arc,
};
use tracing::{debug, error, info, warn};
use ursa_index_provider::{
    advertisement::{Advertisement, MAX_ENTRIES},
    provider::{Provider, ProviderInterface},
};
use ursa_metrics::events::{track, MetricEvent};
use ursa_store::{BitswapStorage, Dag, Store};

use crate::{
    behaviour::{Behaviour, BehaviourEvent, BitswapInfo, BlockSenderChannel},
    codec::protocol::{UrsaExchangeRequest, UrsaExchangeResponse},
    transport::UrsaTransport,
    NetworkConfig,
};
use metrics::Label;
use ursa_utils::convert_cid;

pub const URSA_GLOBAL: &str = "/ursa/global";
pub const MESSAGE_PROTOCOL: &[u8] = b"/ursa/message/0.0.1";

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

    StartProviding {
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
    /// index provider
    index_provider: Provider<S>,
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

        let transport = UrsaTransport::new(&keypair, config, relay_transport);

        let bitswap_store = BitswapStorage(store.clone());

        let behaviour = Behaviour::new(&keypair, config, bitswap_store, relay_client);

        let limits = ConnectionLimits::default()
            .with_max_pending_incoming(Some(2 << 9))
            .with_max_pending_outgoing(Some(2 << 9))
            .with_max_established_incoming(Some(2 << 9))
            .with_max_established_outgoing(Some(2 << 9))
            .with_max_established_per_peer(Some(8));

        let mut swarm = SwarmBuilder::new(transport, behaviour, local_peer_id)
            .notify_handler_buffer_size(NonZeroUsize::new(2 << 7).unwrap())
            .connection_event_buffer_size(2 << 7)
            .dial_concurrency_factor(NonZeroU8::new(8).unwrap())
            .connection_limits(limits)
            .executor(Box::new(|future| {
                task::spawn(future);
            }))
            .build();

        swarm.listen_on(config.swarm_addr.clone()).unwrap();

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

        let (event_sender, event_receiver) = unbounded();
        let (command_sender, command_receiver) = unbounded();

        UrsaService {
            swarm,
            store,
            command_sender,
            command_receiver,
            event_sender,
            event_receiver,
            response_channels: Default::default(),
            index_provider,
        }
    }

    pub fn command_sender(&self) -> &Sender<UrsaCommand> {
        &self.command_sender
    }
    /// Start the ursa network service loop.
    ///
    /// Poll `swarm` and `command_receiver` from [`UrsaService`].
    /// - `swarm` handles the network events [Event].
    /// - `command_receiver` handles inbound commands [Command].
    pub async fn start(mut self) -> Result<()> {
        let peer_id = *self.swarm.local_peer_id();
        let provider = self.index_provider;

        info!("Node starting up with peerId {:?}", peer_id);

        let mut swarm = self.swarm.fuse();
        let mut blockstore = BitswapStorage(self.store.clone());
        let mut command_receiver = self.command_receiver.fuse();

        loop {
            select! {
                event = swarm.next() => {
                    if let Some(event) = event {
                        match event {
                            SwarmEvent::Behaviour(event) => match event {
                                BehaviourEvent::Bitswap(BitswapInfo {cid, query_id, block_found })=> {
                                    swarm.get_mut().behaviour_mut().cancel(query_id);
                                    let labels = vec![
                                        Label::new("cid", format!("{}", cid)),
                                        Label::new("query_id", format!("{}", query_id)),
                                        Label::new("block_found", format!("{}", block_found)),
                                    ];

                                    track(MetricEvent::Bitswap, Some(labels), None);

                                    if let Some (chans) = self.response_channels.remove(&cid) {
                                        // TODO: in some cases, the insert takes few milliseconds after query complete is received
                                        // wait for block to be inserted
                                        let bitswap_cid = convert_cid(cid.to_bytes());
                                        if let true = block_found { loop { if blockstore.contains(&bitswap_cid).unwrap() { break; } } }

                                        for chan in chans.into_iter(){
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
                    },
                                BehaviourEvent::GossipMessage {
                                    peer,
                                    topic,
                                    message,
                                } => {
                                    debug!("[BehaviourEvent::Gossip] - received from {:?}", peer);
                                    let swarm_mut = swarm.get_mut();
                                    let labels =  vec![
                                        Label::new("peer", format!("{}", peer)),
                                        Label::new("topic", format!("{}", topic)),
                                        Label::new("message", format!("{:?}", message)),
                                    ];

                                    track(MetricEvent::GossipMessage, Some(labels), None);

                                    if swarm_mut.is_connected(&peer) {
                                        let status = self
                                            .event_sender
                                            .send(UrsaEvent::GossipsubMessage(message))
                                            .await;

                                        if status.is_err()
                                        {
                                            warn!("[BehaviourEvent::Gossip] - failed to publish message to topic: {:?}", topic);
                                        }
                                    }
                                },
                                BehaviourEvent::RequestMessage { peer, request, channel } => {
                                    debug!("[BehaviourEvent::RequestMessage] {} ", peer);
                                    let labels = vec![
                                        Label::new("peer", format!("{}", peer)),
                                        Label::new("request", format!("{:?}", request)),
                                        Label::new("channel", format!("{:?}", channel)),
                                    ];

                                    track(MetricEvent::RequestMessage, Some(labels), None);

                                    if self
                                        .event_sender
                                        .send(UrsaEvent::RequestMessage { request, channel })
                                        .await
                                        .is_err()
                                    {
                                        warn!("[BehaviourEvent::RequestMessage] - failed to send request to peer: {:?}", peer);
                                    }
                                },
                                BehaviourEvent::PeerConnected(peer) => {
                                    debug!("[BehaviourEvent::PeerConnected] - Peer connected {:?}", peer);

                                    track(MetricEvent::PeerConnected, None, None);

                                    if self
                                        .event_sender
                                        .send(UrsaEvent::PeerConnected(peer))
                                        .await
                                        .is_err()
                                    {
                                        warn!("[BehaviourEvent::PeerConnected] - failed to send peer connection message: {:?}", peer);
                                    }
                                }
                                BehaviourEvent::PeerDisconnected(peer) => {
                                    debug!("[BehaviourEvent::PeerDisconnected] - Peer disconnected {:?}", peer);

                                    track(MetricEvent::PeerDisconnected, None, None);

                                    if self
                                        .event_sender
                                        .send(UrsaEvent::PeerDisconnected(peer))
                                        .await
                                        .is_err()
                                    {
                                        warn!("[BehaviourEvent::PeerDisconnected] - failed to send peer disconnect message: {:?}", peer);
                                    }
                                }
                                BehaviourEvent::PublishAd { root_cid, context_id, is_rm } => {
                                    let root_cid = Cid::try_from(root_cid).expect("Cid from bytes failed");

                                    debug!("creating advertisement for cids under root cid: {:?}", root_cid);
                                    let addresses: Vec<String> = swarm.get_mut().listeners().cloned().map(|m| m.to_string()).collect();

                                    let ad = Advertisement::new(context_id.clone(), peer_id, addresses, is_rm);
                                    let id = provider.create(ad).await.unwrap();

                                    let dag = self.store.dag_traversal(&(convert_cid(root_cid.to_bytes())))?;
                                    let entries = dag.iter().map(|d| return Ipld::Bytes(d.0.hash().to_bytes())).collect::<Vec<Ipld>>();
                                    let chunks: Vec<&[Ipld]> = entries.chunks(MAX_ENTRIES).collect();

                                    debug!("inserting the chunks");
                                    for chunk in chunks.iter() {
                                        let entries_bytes = forest_encoding::to_vec(&chunk)?;
                                        provider.add_chunk(entries_bytes, id).await.expect(" adding chunk to ad should not fail");
                                    }
                                    debug!("Publishing the advertisement now");
                                    provider.publish(id).await.expect("publishing the ad should not fail");
                                    let announce_msg = provider.announce_msg(peer_id).await.unwrap();

                                    let i_topic_hash = TopicHash::from_raw("indexer/ingest/mainnet");
                                    let i_topic = Topic::new("indexer/ingest/mainnet");
                                    let g_msg = GossipsubMessage {data:announce_msg, source: None, sequence_number: None, topic: i_topic_hash };
                                    match swarm.get_mut().behaviour_mut().publish(i_topic, g_msg.clone()) {
                                        Ok(res) => {},
                                        Err(e) => {
                                            error!("there was an error while gossiping the announcement");
                                            error!("{:?}", e);
                                        }
                                    }
                                }
                                BehaviourEvent::NatStatusChanged{ old, new } => {
                                    let swarm = swarm.get_mut();

                                    match (old, new) {
                                        (NatStatus::Unknown, NatStatus::Private) => {
                                            let behaviour = swarm.behaviour_mut();
                                            if behaviour.is_relay_client_enabled() {
                                                // get random bootstrap node and listen on their relay
                                                if let Some((relay_peer, relay_addr)) = behaviour
                                                    .discovery()
                                                    .bootstrap_addrs()
                                                    .choose(&mut rand::thread_rng())
                                                {
                                                    let addr = relay_addr.clone().with(Protocol::P2p((*relay_peer).into())).with(Protocol::P2pCircuit);
                                                    warn!("Private NAT detected. Establishing public relay address on peer {}", addr);
                                                    swarm.listen_on(addr)
                                                        .expect("failed to listen on relay");
                                                }
                                            }
                                        },
                                        (_, NatStatus::Public(addr)) => {
                                            info!("Public Nat verified! Public listening address: {}", addr);
                                        },
                                        (old, new) => {
                                            warn!("NAT status changed from {:?} to {:?}", old, new);
                                        }
                                    }
                                }
                                BehaviourEvent::RelayReservationOpened { peer_id } => {
                                    debug!("Relay reservation opened for peer {}", peer_id);
                                    track(MetricEvent::RelayReservationOpened, None, None);
                                }
                                BehaviourEvent::RelayReservationClosed { peer_id } => {
                                    debug!("Relay reservation closed for peer {}", peer_id);
                                    track(MetricEvent::RelayReservationClosed, None, None);
                                }
                                BehaviourEvent::RelayCircuitOpened => {
                                    debug!("Relay circuit opened");
                                    track(MetricEvent::RelayCircuitOpened, None, None);
                                }
                                BehaviourEvent::RelayCircuitClosed => {
                                    debug!("Relay circuit closed");
                                    track(MetricEvent::RelayCircuitClosed, None, None);
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
                            | SwarmEvent::OutgoingConnectionError { .. } => {},
                        }
                    }
                },
                command = command_receiver.next() => {
                    if let Some(command) = command {
                        match command {
                            UrsaCommand::GetBitswap { cid, query, sender } => {
                                let peers = swarm.get_mut().behaviour_mut().peers();
                                if peers.is_empty() {
                                    error!("There were no peers provided and the block does not exist in local store");
                                    let _ = sender.send(Err(anyhow!("There were no peers provided and the block does not exist in local store")));
                                }
                                else {
                                    if let Some(chans) = self.response_channels.get_mut(&cid) {
                                        chans.push(sender);
                                    } else {
                                        self.response_channels.insert(cid, vec![sender]);
                                    }
                                    match query{
                                        BitswapType::Get => swarm.get_mut().behaviour_mut().get_block(cid, peers.iter().copied()),
                                        BitswapType::Sync => swarm.get_mut().behaviour_mut().sync_block(cid, peers.into_iter().collect()),
                                    }

                                }
                            },
                            UrsaCommand::Put { cid, sender } => {},
                            UrsaCommand::GetPeers { sender } => {
                                let peers = swarm.get_mut().behaviour_mut().peers();
                                let _ = sender.send(peers).map_err(|_| anyhow!("Failed to get Libp2p peers"));
                            }
                            UrsaCommand::StartProviding { cids, sender } => {
                                // TODO: start providing via gossip and/or publish ad to the indexer
                                let _ = swarm.get_mut().behaviour_mut().publish_ad(cids.clone());
                                let _channel = sender.send(Ok(cids));
                            },
                            UrsaCommand::SendRequest { peer_id, request, channel } => {
                                let _ = swarm.get_mut().behaviour_mut().send_request(peer_id, request, channel);
                            },
                            UrsaCommand::SendResponse { request_id, response, channel } => todo!(),
                            UrsaCommand::GossipsubMessage { topic, message } => {
                                if let Err(error) = swarm.get_mut().behaviour_mut().publish(topic.clone(), message.clone()) {
                                    warn!(
                                        "[UrsaCommand::GossipsubMessage] - Failed to publish message to topic {:?} with error {:?}:",
                                        URSA_GLOBAL, error
                                    );
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::codec::protocol::RequestType;
    use async_std::{fs::File, io::BufReader, sync::RwLock};
    use db::{rocks::RocksDb, rocks_config::RocksDbConfig};
    use fvm_ipld_car::{load_car, CarReader};
    use libipld::{cbor::DagCborCodec, ipld, multihash::Code, Block, DefaultParams, Ipld};
    use simple_logger::SimpleLogger;
    use ursa_index_provider::config::ProviderConfig;
    use std::{str::FromStr, thread, time::Duration, vec};
    use tracing::log::LevelFilter;
    use ursa_store::Store;

    fn create_block(ipld: Ipld) -> Block<DefaultParams> {
        Block::encode(DagCborCodec, Code::Blake3_256, &ipld).unwrap()
    }

    fn network_init(
        config: &mut NetworkConfig,
        store: Arc<Store<RocksDb>>,
    ) -> (UrsaService<RocksDb>, PeerId) {
        let keypair = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(keypair.public());
        config.bootstrap_nodes = ["/ip4/127.0.0.1/tcp/6009"]
            .iter()
            .map(|node| node.parse().unwrap())
            .collect();

        let config = NetworkConfig::default();
        let provider_db = RocksDb::open("index_provider_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");
            let provider_config = ProviderConfig::default();
            let index_provider = Provider::new(keypair.clone(), Arc::new(RwLock::new(provider_db)), provider_config.clone());
    
        let service =
            UrsaService::new(keypair, &config, Arc::clone(&store), index_provider.clone());

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

    // Network Starts
    #[test]
    fn test_network_start() {
        setup_logger(LevelFilter::Debug);

        let db = RocksDb::open("test_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");
        let db = Arc::new(db);
        let store = Arc::new(Store::new(Arc::clone(&db)));

        let (service, _) = network_init(&mut NetworkConfig::default(), Arc::clone(&store));

        task::spawn(async {
            if let Err(err) = service.start().await {
                error!("[service_task] - {:?}", err);
            }
        });
    }

    #[async_std::test]
    async fn test_network_gossip() {
        setup_logger(LevelFilter::Debug);
        let mut config = NetworkConfig::default();
        let topic = Topic::new(URSA_GLOBAL);

        let db = RocksDb::open("test_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");
        let db = Arc::new(db);
        let store = Arc::new(Store::new(Arc::clone(&db)));

        let (node_1, _) = network_init(&mut config, Arc::clone(&store));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let (node_2, _) = network_init(&mut config, Arc::clone(&store));

        let node_1_sender = node_1.command_sender.clone();
        let node_2_receiver = node_2.event_receiver.clone();

        task::spawn(async {
            if let Err(err) = node_1.start().await {
                error!("[service_task] - {:?}", err);
            }
        });

        task::spawn(async {
            if let Err(err) = node_2.start().await {
                error!("[service_task] - {:?}", err);
            }
        });

        let delay = Duration::from_millis(2000);
        thread::sleep(delay);

        let msg = UrsaCommand::GossipsubMessage {
            topic: topic.clone(),
            message: GossipsubMessage {
                source: None,
                data: vec![1],
                sequence_number: Some(1),
                topic: topic.hash(),
            },
        };
        node_1_sender.send(msg).await.unwrap();

        let mut command_receiver = node_2_receiver.fuse();

        loop {
            if let Some(UrsaEvent::GossipsubMessage(gossip)) = command_receiver.next().await {
                assert_eq!(vec![1], gossip.data);
                break;
            }
        }
    }

    #[async_std::test]
    async fn test_network_mdns() {
        setup_logger(LevelFilter::Debug);
        let mut config = NetworkConfig {
            mdns: true,
            ..Default::default()
        };

        let db = RocksDb::open("test_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");
        let db = Arc::new(db);
        let store = Arc::new(Store::new(Arc::clone(&db)));

        let (node_1, _) = network_init(&mut config, Arc::clone(&store));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let (node_2, _) = network_init(&mut config, Arc::clone(&store));

        task::spawn(async {
            if let Err(err) = node_1.start().await {
                error!("[service_task] - {:?}", err);
            }
        });

        let mut swarm_2 = node_2.swarm.fuse();

        loop {
            if let Some(SwarmEvent::Behaviour(BehaviourEvent::PeerConnected(peer_id))) =
                swarm_2.next().await
            {
                info!("Node 2 PeerConnected: {:?}", peer_id);
                break;
            }
        }
    }

    #[async_std::test]
    async fn test_network_discovery() {
        setup_logger(LevelFilter::Debug);
        let mut config = NetworkConfig::default();

        let db = RocksDb::open("test_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");
        let db = Arc::new(db);
        let store = Arc::new(Store::new(Arc::clone(&db)));

        let (node_1, _) = network_init(&mut config, Arc::clone(&store));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let (node_2, _) = network_init(&mut config, Arc::clone(&store));

        task::spawn(async {
            if let Err(err) = node_1.start().await {
                error!("[service_task] - {:?}", err);
            }
        });

        let mut swarm_2 = node_2.swarm.fuse();

        loop {
            if let Some(SwarmEvent::Behaviour(BehaviourEvent::PeerConnected(peer_id))) =
                swarm_2.next().await
            {
                info!("Node 2 PeerConnected: {:?}", peer_id);
                break;
            }
        }
    }

    #[async_std::test]
    async fn test_network_req_res() {
        setup_logger(LevelFilter::Debug);
        let mut config = NetworkConfig::default();

        let db = RocksDb::open("test_db", &RocksDbConfig::default())
            .expect("Opening RocksDB must succeed");
        let db = Arc::new(db);
        let store = Arc::new(Store::new(Arc::clone(&db)));

        let (node_1, _) = network_init(&mut config, Arc::clone(&store));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let (node_2, peer_2) = network_init(&mut config, Arc::clone(&store));

        let node_1_sender = node_1.command_sender.clone();

        task::spawn(async {
            if let Err(err) = node_1.start().await {
                error!("[service_task] - {:?}", err);
            }
        });

        let delay = Duration::from_millis(2000);
        thread::sleep(delay);

        let (sender, _) = oneshot::channel();
        let request = UrsaExchangeRequest(RequestType::CarRequest("Qm".to_string()));
        let msg = UrsaCommand::SendRequest {
            peer_id: peer_2,
            request,
            channel: sender,
        };

        node_1_sender.send(msg).await.unwrap();

        let mut swarm_2 = node_2.swarm.fuse();

        loop {
            if let Some(SwarmEvent::Behaviour(BehaviourEvent::RequestMessage { request, .. })) =
                swarm_2.next().await
            {
                info!("Node 2 RequestMessage: {:?}", request);
                break;
            }
        }
    }

    #[async_std::test]
    async fn test_bitswap_get() {
        setup_logger(LevelFilter::Info);
        let mut config = NetworkConfig::default();

        let store1 = get_store("test_db1");
        let store2 = get_store("test_db2");

        let bitswap_store_1 = BitswapStorage(store1.clone());
        let mut bitswap_store_2 = BitswapStorage(store2.clone());

        let block = get_block(&b"hello world"[..]);
        info!("inserting block into bitswap store for node 1");
        insert_block(bitswap_store_1, &block);

        let (node_1, _) = network_init(&mut config, Arc::clone(&store1));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let (node_2, _) = network_init(&mut config, Arc::clone(&store2));

        let node_2_sender = node_2.command_sender.clone();

        task::spawn(async {
            if let Err(err) = node_1.start().await {
                error!("[service_task] - {:?}", err);
            }
        });

        task::spawn(async {
            if let Err(err) = node_2.start().await {
                error!("[service_task] - {:?}", err);
            }
        });

        let delay = Duration::from_millis(2000);
        thread::sleep(delay);

        let (sender, receiver) = oneshot::channel();
        let msg = UrsaCommand::GetBitswap {
            cid: convert_cid(block.cid().to_bytes()),
            query: BitswapType::Get,
            sender,
        };
        node_2_sender.send(msg).await.unwrap();

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
    }

    #[async_std::test]
    async fn test_bitswap_get_block_not_found() {
        setup_logger(LevelFilter::Info);
        let mut config = NetworkConfig::default();

        let store1 = get_store("test_db1");
        let store2 = get_store("test_db2");

        let (node_1, _) = network_init(&mut config, Arc::clone(&store1));

        let block = get_block(&b"hello world"[..]);

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let (node_2, _) = network_init(&mut config, Arc::clone(&store2));

        let node_2_sender = node_2.command_sender.clone();

        task::spawn(async {
            if let Err(err) = node_1.start().await {
                error!("[service_task] - {:?}", err);
            }
        });

        task::spawn(async {
            if let Err(err) = node_2.start().await {
                error!("[service_task] - {:?}", err);
            }
        });

        let delay = Duration::from_millis(2000);
        thread::sleep(delay);

        let (sender, receiver) = oneshot::channel();

        let msg = UrsaCommand::GetBitswap {
            cid: convert_cid(block.cid().to_bytes()),
            query: BitswapType::Get,
            sender,
        };
        node_2_sender.send(msg).await.unwrap();

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
    }

    #[async_std::test]
    async fn add_block() {
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
        info!("{:?}", bitswap_store.contains(&convert_cid(cid.to_bytes())))
    }

    #[async_std::test]
    async fn get_block_local() {
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
    }

    #[async_std::test]
    async fn test_bitswap_sync() -> Result<()> {
        setup_logger(LevelFilter::Info);
        let mut config = NetworkConfig::default();

        let store1 = get_store("test_db1");
        let store2 = get_store("test_db2");

        let mut bitswap_store2 = BitswapStorage(store2.clone());

        let path = "../car_files/text_mb.car";

        // put the car file in store 1
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

        let (node_1, _) = network_init(&mut config, Arc::clone(&store1));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let (node_2, _) = network_init(&mut config, Arc::clone(&store2));
        let node_2_sender = node_2.command_sender.clone();

        task::spawn(async {
            if let Err(err) = node_1.start().await {
                error!("[service_task] - {:?}", err);
            }
        });

        task::spawn(async {
            if let Err(err) = node_2.start().await {
                error!("[service_task] - {:?}", err);
            }
        });

        let delay = Duration::from_millis(2000);
        thread::sleep(delay);

        let (sender, receiver) = oneshot::channel();

        let msg = UrsaCommand::GetBitswap {
            cid: cids[0],
            query: BitswapType::Sync,
            sender,
        };
        node_2_sender.send(msg).await.unwrap();

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
