//! # Ursa libp2p implementation.
//!
//! The service is bootstrapped with the following premises:
//!
//! - Load or create a new [`Keypair`] by checking the local storage.
//! - Instanitate the [`UrsaTransport`] module with quic.or(tcp) and relay support.
//! - A custome ['NetworkBehaviour'] is implemented based on [`UrsaConfig`] provided by node runner.
//! - Using the [`UrsaTransport`] and [`Behaviour`] a new [`Swarm`] is built.
//! - Two channels are created to serve (send/recieve) both the network [`UrsaCommand`]'s and [`UrsaEvent`]'s.
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
use futures::{channel::oneshot, select};
use futures_util::stream::StreamExt;
use ipld_blockstore::BlockStore;
use libipld::DefaultParams;
use libp2p::{
    gossipsub::{GossipsubMessage, IdentTopic as Topic},
    identity::Keypair,
    request_response::ResponseChannel,
    swarm::{ConnectionLimits, SwarmBuilder, SwarmEvent},
    PeerId, Swarm,
};
use libp2p_bitswap::{BitswapEvent, BitswapStore};
use std::{collections::HashSet, sync::Arc};
use store::{BitswapStorage, Store};
use tracing::{debug, error, info, warn};

use crate::{
    behaviour::{Behaviour, BehaviourEvent, BitswapInfo, BlockSenderChannel},
    codec::protocol::{UrsaExchangeRequest, UrsaExchangeResponse},
    config::UrsaConfig,
    transport::UrsaTransport,
    utils,
};

pub const URSA_GLOBAL: &str = "/ursa/global";
pub const MESSAGE_PROTOCOL: &[u8] = b"/ursa/message/0.0.1";

#[derive(Debug)]
pub enum UrsaCommand {
    /// Rpc commands
    Get {
        cid: Cid,
        sender: BlockSenderChannel,
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
        topic: Topic,
        message: GossipsubMessage,
    },
}

#[derive(Debug)]
pub enum UrsaEvent {
    /// An event trigger when remote peer connects.
    PeerConnected(PeerId),
    /// An event trigger when remote peer disconnects.
    PeerDisconnected(PeerId),
    BitswapEvent(BitswapEvent),
    /// A Gossip message request was recieved from a peer.
    GossipsubMessage(GossipsubMessage),
    /// A message request was recieved from a peer.
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
    response_channels: FnvHashMap<Cid, Vec<BlockSenderChannel>>,
}

impl<S> UrsaService<S>
where
    S: BlockStore + Sync + Send + 'static,
{
    /// Init a new [`UrsaService`] based on [`UrsaConfig`]
    ///
    /// For ursa `keypair` we use ed25519 either
    /// checking for a local store or creating a new keypair.
    ///
    /// For ursa `transport` we build a default QUIC layer and
    /// failover to tcp.
    ///
    /// For ursa behaviour we use [`Behaviour`].
    ///
    /// We construct a [`Swarm`] with [`UrsaTransport`] and [`Behaviour`]
    /// listening on [`UrsaConfig`] `swarm_addr`.
    ///
    pub fn new(keypair: Keypair, config: &UrsaConfig, store: Arc<Store<S>>) -> Self {
        let local_peer_id = PeerId::from(keypair.public());

        info!(target: "ursa-libp2p", "Node identity is: {}", local_peer_id.to_base58());

        let transport = UrsaTransport::new(&keypair, config);

        let bitswap_store = BitswapStorage(store.clone());

        let behaviour = Behaviour::new(&keypair, config, bitswap_store);

        let limits = ConnectionLimits::default()
            .with_max_pending_incoming(Some(10))
            .with_max_pending_outgoing(Some(10))
            .with_max_established_incoming(Some(10))
            .with_max_established_outgoing(Some(10))
            .with_max_established(Some(10))
            .with_max_established_per_peer(Some(10));

        let mut swarm = SwarmBuilder::new(transport, behaviour, local_peer_id)
            // .notify_handler_buffer_size(todo!())
            // .connection_event_buffer_size(todo!())
            .connection_limits(limits)
            .executor(Box::new(|future| {
                task::spawn(future);
            }))
            .build();

        Swarm::listen_on(&mut swarm, config.swarm_addr.clone()).unwrap();

        for to_dial in &config.bootstrap_nodes {
            Swarm::dial(&mut swarm, to_dial.clone())
                .map_err(|err| anyhow!("{}", err))
                .unwrap();
        }

        for to_dial in &config.bootstrap_nodes {
            Swarm::dial(&mut swarm, to_dial.clone())
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
        info!(
            "Node startig up with peerid {:?}",
            self.swarm.local_peer_id()
        );
        let mut swarm = self.swarm.fuse();
        let mut blockstore = BitswapStorage(self.store.clone());
        let mut command_receiver = self.command_receiver.fuse();

        loop {
            select! {
                event = swarm.next() => {
                    if let Some(event) = event {
                        match event {
                            SwarmEvent::Behaviour(event) => match event {
                                BehaviourEvent::Bitswap(info)=> {
                                    let BitswapInfo {cid, query_id, block_found } = info;
                                    swarm.get_mut().behaviour_mut().cancel(query_id);
                                    if let Some (chans) = self.response_channels.remove(&cid) {
                                        // TODO: in some cases, the insert takes few milliseconds after query complete is received
                                        // wait for block to be inserted
                                        let bitswap_cid = utils::convert_cid(cid.to_bytes());
                                        if let true = block_found { loop { if blockstore.contains(&bitswap_cid).unwrap() { break; } } }

                                        for chan in chans.into_iter(){
                                            if let Ok(Some(data)) = blockstore.get(&bitswap_cid) {
                                                if chan.send(Ok(data)).is_err() {
                                                    error!("[BehaviourEvent::Bitswap] - Bitswap response channel send failed");
                                                }
                                                info!("[BehaviourEvent::Bitswap] - Send Bitswap block with cid {:?}, via oneshot channel", cid);
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

                                    if swarm_mut.is_connected(&peer) {
                                        if self
                                            .event_sender
                                            .send(UrsaEvent::GossipsubMessage(message))
                                            .await
                                            .is_err()
                                        {
                                            warn!("[BehaviourEvent::Gossip] - failed to publish message to topic: {:?}", topic);
                                        }
                                    }
                                },
                                BehaviourEvent::RequestMessage { peer, request, channel } => {
                                    debug!("[BehaviourEvent::RequestMessage] - Peer connected {:?}", peer);

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
                                    if self
                                        .event_sender
                                        .send(UrsaEvent::PeerDisconnected(peer))
                                        .await
                                        .is_err()
                                    {
                                        warn!("[BehaviourEvent::PeerDisconnected] - failed to send peer disconnect message: {:?}", peer);
                                    }
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
                            UrsaCommand::Get { cid, sender } => {
                                if let Ok(Some(data)) = blockstore.get(&utils::convert_cid(cid.to_bytes())) {
                                    let _ = sender.send(Ok(data));
                                } else {
                                    if let Some(chans) = self.response_channels.get_mut(&cid) {
                                        chans.push(sender);
                                    } else {
                                        self.response_channels.insert(cid, vec![sender]);
                                    }
                                    let peers = swarm.get_mut().behaviour_mut().peers();
                                    swarm.get_mut().behaviour_mut().get_block(cid, peers.iter().copied());
                                }

                            },
                            UrsaCommand::Put { cid, sender } => {},
                            UrsaCommand::GetPeers { sender } => {
                                let peers = swarm.get_mut().behaviour_mut().peers();
                                let _ = sender.send(peers).map_err(|_| anyhow!("Failed to get Libp2p peers"));
                            }
                            UrsaCommand::SendRequest { peer_id, request, channel } => {
                                let _ = swarm.get_mut().behaviour_mut().send_request(peer_id, request, channel);
                            },
                            UrsaCommand::GossipsubMessage { topic, message } => {
                                if let Err(error) = swarm.get_mut().behaviour_mut().publish(topic.clone(), message.clone()) {
                                    warn!(
                                        "[UrsaCommand::GossipsubMessage] - Failed to publish message top topic {:?} with error {:?}:",
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
    use db::rocks::RocksDb;
    use libipld::{cbor::DagCborCodec, ipld, multihash::Code, Block, DefaultParams, Ipld};
    use log::LevelFilter;
    use simple_logger::SimpleLogger;
    use std::{thread, time::Duration, vec};
    use store::Store;

    fn create_block(ipld: Ipld) -> Block<DefaultParams> {
        Block::encode(DagCborCodec, Code::Blake3_256, &ipld).unwrap()
    }

    fn network_init(
        config: &UrsaConfig,
        store: Arc<Store<RocksDb>>,
    ) -> (UrsaService<RocksDb>, PeerId) {
        let keypair = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(keypair.public());

        let service = UrsaService::new(keypair, config, store);

        (service, local_peer_id)
    }

    // Network Starts
    #[test]
    fn test_network_start() {
        SimpleLogger::new()
            .with_utc_timestamps()
            .with_colors(true)
            .init()
            .unwrap();

        let db = RocksDb::open("test_db").expect("Opening RocksDB must succeed");
        let db = Arc::new(db);
        let store = Arc::new(Store::new(Arc::clone(&db)));

        let (service, _) = network_init(&UrsaConfig::default(), Arc::clone(&store));

        task::spawn(async {
            if let Err(err) = service.start().await {
                error!("[service_task] - {:?}", err);
            }
        });
    }

    #[async_std::test]
    async fn test_network_gossip() {
        SimpleLogger::new().with_utc_timestamps().init().unwrap();
        let mut config = UrsaConfig::default();
        let topic = Topic::new(URSA_GLOBAL);

        let db = RocksDb::open("test_db").expect("Opening RocksDB must succeed");
        let db = Arc::new(db);
        let store = Arc::new(Store::new(Arc::clone(&db)));

        let (node_1, _) = network_init(&config, Arc::clone(&store));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let (node_2, _) = network_init(&config, Arc::clone(&store));

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
        SimpleLogger::new().with_utc_timestamps().init().unwrap();
        let mut config = UrsaConfig {
            mdns: true,
            ..Default::default()
        };

        let db = RocksDb::open("test_db").expect("Opening RocksDB must succeed");
        let db = Arc::new(db);
        let store = Arc::new(Store::new(Arc::clone(&db)));

        let (node_1, _) = network_init(&config, Arc::clone(&store));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let (node_2, _) = network_init(&config, Arc::clone(&store));

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
        SimpleLogger::new().with_utc_timestamps().init().unwrap();
        let mut config = UrsaConfig::default();

        let db = RocksDb::open("test_db").expect("Opening RocksDB must succeed");
        let db = Arc::new(db);
        let store = Arc::new(Store::new(Arc::clone(&db)));

        let (node_1, _) = network_init(&config, Arc::clone(&store));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let (node_2, _) = network_init(&config, Arc::clone(&store));

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
        SimpleLogger::new().with_utc_timestamps().init().unwrap();
        let mut config = UrsaConfig::default();

        let db = RocksDb::open("test_db").expect("Opening RocksDB must succeed");
        let db = Arc::new(db);
        let store = Arc::new(Store::new(Arc::clone(&db)));

        let (node_1, _) = network_init(&config, Arc::clone(&store));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let (node_2, peer_2) = network_init(&config, Arc::clone(&store));

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
        SimpleLogger::new()
            .with_level(LevelFilter::Info)
            .with_utc_timestamps()
            .init()
            .unwrap();
        let mut config = UrsaConfig::default();

        let db1 = Arc::new(RocksDb::open("test_db1").expect("Opening RocksDB must succeed"));
        let db2 = Arc::new(RocksDb::open("test_db2").expect("Opening RocksDB must succeed"));

        let store1 = Arc::new(Store::new(Arc::clone(&db1)));
        let store2 = Arc::new(Store::new(Arc::clone(&db2)));

        let mut bitswap_store_1 = BitswapStorage(store1.clone());

        let (node_1, _) = network_init(&config, Arc::clone(&store1));

        let block = create_block(ipld!(&b"hello world"[..]));
        info!("inserting block into bitswap store for node 1");
        if let Err(err) = bitswap_store_1.insert(&block) {
            error!(
                "there was an error while inserting into the blockstore {:?}",
                err
            );
        } else {
            info!("block inserted successfully");
        }

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let (node_2, _) = network_init(&config, Arc::clone(&store2));

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
        let msg = UrsaCommand::Get {
            cid: utils::convert_cid(block.cid().to_bytes()),
            sender,
        };
        node_2_sender.send(msg).await.unwrap();

        futures::executor::block_on(async {
            info!("waiting for msg on block receive channel...");
            let value = receiver.await.expect("Unable to receive from channel");
            match value {
                Ok(val) => assert_eq!(val, block.data()),
                _ => {}
            }
        });
    }

    #[async_std::test]
    async fn test_bitswap_get_block_not_found() {
        SimpleLogger::new()
            .with_level(LevelFilter::Info)
            .with_utc_timestamps()
            .init()
            .unwrap();
        let mut config = UrsaConfig::default();

        let db1 = Arc::new(RocksDb::open("test_db1").expect("Opening RocksDB must succeed"));
        let db2 = Arc::new(RocksDb::open("test_db2").expect("Opening RocksDB must succeed"));

        let store1 = Arc::new(Store::new(Arc::clone(&db1)));
        let store2 = Arc::new(Store::new(Arc::clone(&db2)));

        let (node_1, _) = network_init(&config, Arc::clone(&store1));

        let block = create_block(ipld!(&b"hello world"[..]));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let (node_2, _) = network_init(&config, Arc::clone(&store2));

        let node_2_sender = node_2.command_sender.clone();
        // let node_2_receiver = node_2.event_receiver.clone();

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

        let msg = UrsaCommand::Get {
            cid: utils::convert_cid(block.cid().to_bytes()),
            sender,
        };
        node_2_sender.send(msg).await.unwrap();

        futures::executor::block_on(async {
            info!("waiting for msg on block receive channel...");
            let value = receiver.await.expect("Unable to receive from channel");
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
}
