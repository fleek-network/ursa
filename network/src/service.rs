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
use libp2p_bitswap::BitswapEvent;
use std::{collections::HashSet, sync::Arc};
use store::{BitswapStorage, Store};
use tiny_cid::Cid;
use tracing::{debug, info, warn};

use crate::{
    behaviour::{Behaviour, BehaviourEvent, BehaviourEventError},
    codec::protocol::{UrsaExchangeRequest, UrsaExchangeResponse},
    config::UrsaConfig,
    transport::UrsaTransport,
};

pub const URSA_GLOBAL: &str = "/ursa/global";
pub const MESSAGE_PROTOCOL: &[u8] = b"/ursa/message/0.0.1";

#[derive(Debug)]
pub enum UrsaCommand {
    /// Rpc commands
    Get {
        cid: Cid,
        sender: oneshot::Sender<HashSet<PeerId>>,
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
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    BitswapEvent(BitswapEvent),
    GossipsubMessage(GossipsubMessage),
    RequestMessage {
        request: UrsaExchangeRequest,
        channel: ResponseChannel<UrsaExchangeResponse>,
    },
}

pub struct UrsaService<S> {
    /// Store
    store: Arc<Store<S>>,
    /// The main libp2p swamr emitting events.
    swarm: Swarm<Behaviour<DefaultParams>>,
    /// Handles outbound messages to peers
    command_sender: Sender<UrsaCommand>,
    /// Handles inbound messages from peers
    command_receiver: Receiver<UrsaCommand>,
    /// Handles events emitted by the ursa network
    event_sender: Sender<UrsaEvent>,
    /// Handles events received by the ursa network
    event_receiver: Receiver<UrsaEvent>,
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
    pub fn new(config: &UrsaConfig, store: Arc<Store<S>>) -> Self {
        // Todo: Create or get from local store
        let keypair = Keypair::generate_ed25519();
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
        }
    }

    /// Start the ursa network service loop.
    ///
    /// Poll `swarm` and `command_receiver` from [`UrsaService`].
    /// - `swarm` handles the network events [Event].
    /// - `command_receiver` handles inbound commands [Command].
    pub async fn start(self) {
        let mut swarm = self.swarm.fuse();
        let mut command_receiver = self.command_receiver.fuse();

        loop {
            select! {
                event = swarm.next() => {
                    if let Some(event) = event {
                        match event {
                            SwarmEvent::Behaviour(event) => match event {
                                BehaviourEvent::Bitswap(_) => {},
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
                            UrsaCommand::Get { cid, sender } => {},
                            UrsaCommand::Put { cid, sender } => {},
                            UrsaCommand::GetPeers { sender } => {
                                let peers = swarm.get_mut().behaviour_mut().peers();
                                let _ = sender.send(peers).map_err(|_| anyhow!("Failed to get Libp2p peers"));
                            }
                            UrsaCommand::SendRequest { peer_id, request, channel } => {
                                let _ = swarm.get_mut().behaviour_mut().send_request(peer_id, request, channel).await;
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

    use db::rocks::RocksDb;
    use simple_logger::SimpleLogger;
    use std::{thread, time::Duration, vec};
    use store::Store;

    fn network_init(config: UrsaConfig) -> UrsaService<RocksDb> {
        let db = RocksDb::open("test_db").expect("Opening RocksDB must succeed");
        let db = Arc::new(db);

        let store = Arc::new(Store::new(Arc::clone(&db)));
        UrsaService::new(&config, Arc::clone(&store))
    }

    // Network Starts
    #[test]
    fn test_network_start() {
        SimpleLogger::new()
            .with_utc_timestamps()
            .with_colors(true)
            .init()
            .unwrap();
        let service = network_init(UrsaConfig::default());

        task::spawn(async {
            service.start().await;
        });
    }

    // fn test_network_bitswap() {}

    #[async_std::test]
    async fn test_network_gossip() {
        SimpleLogger::new().with_utc_timestamps().init().unwrap();
        let mut config = UrsaConfig::default();
        let topic = Topic::new(URSA_GLOBAL);

        let db = RocksDb::open("test_db").expect("Opening RocksDB must succeed");
        let db = Arc::new(db);
        let store = Arc::new(Store::new(Arc::clone(&db)));

        let node_1 = UrsaService::new(&UrsaConfig::default(), Arc::clone(&store));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let node_2 = UrsaService::new(&config, Arc::clone(&store));

        let node_1_sender = node_1.command_sender.clone();
        let node_2_receiver = node_2.event_receiver.clone();

        task::spawn(async {
            node_1.start().await;
        });

        task::spawn(async {
            node_2.start().await;
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

        let node_1 = UrsaService::new(&UrsaConfig::default(), Arc::clone(&store));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let node_2 = UrsaService::new(&config, Arc::clone(&store));

        task::spawn(async {
            node_1.start().await;
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

        let node_1 = UrsaService::new(&UrsaConfig::default(), Arc::clone(&store));

        config.swarm_addr = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();
        let node_2 = UrsaService::new(&config, Arc::clone(&store));

        task::spawn(async {
            node_1.start().await;
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

    // #[async_std::test]
    // async fn test_network_req_res() {
    //     todo!()
    // }
}
