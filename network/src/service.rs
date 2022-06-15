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

use anyhow::{anyhow, Ok, Result};
use async_std::{
    channel::{unbounded, Receiver, Sender},
    task,
};
use futures::{channel::oneshot, future::ok, select};
use futures_util::stream::StreamExt;
use ipld_blockstore::BlockStore;
use libipld::{store::StoreParams, DefaultParams};
use libp2p::{
    gossipsub::{
        error::PublishError, GossipsubEvent, GossipsubMessage, IdentTopic as Topic, MessageId,
        TopicHash,
    },
    identity::Keypair,
    request_response::RequestResponseEvent,
    swarm::{ConnectionLimits, SwarmBuilder, SwarmEvent},
    PeerId, Swarm,
};
use libp2p_bitswap::BitswapEvent;
use std::{collections::HashSet, marker::PhantomData, sync::Arc};
use store::{BitswapStorage, Store};
use tiny_cid::Cid;
use tracing::{debug, error, info, warn};

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

    /// inter-network commands
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

        let transport = UrsaTransport::new(&keypair, &config);

        let bitswap_store = BitswapStorage(store.clone());

        let behaviour = Behaviour::new(&keypair, &config, bitswap_store);

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
    pub async fn start(mut self) {
        let mut swarm = self.swarm.fuse();
        let mut command_receiver = self.command_receiver.fuse();

        loop {
            select! {
                event = swarm.next() => {
                    if let Some(event) = event {
                        match event {
                            SwarmEvent::Behaviour(event) => match event {
                                BehaviourEvent::Bitswap(_) => {},
                                BehaviourEvent::Gossip {
                                    peer_id,
                                    topic,
                                    message,
                                } => {
                                    debug!("Gossip message received {:?}", peer_id);

                                    // todo(botch)
                                    // Check if topic is correct
                                    // Check the validitay of the message
                                    // report message validation

                                    if self
                                        .event_sender
                                        .send(UrsaEvent::GossipsubMessage(message))
                                        .await
                                        .is_err()
                                    {
                                        warn!("Failed to publish message to topic: {:?}", topic);
                                    }
                                },
                                BehaviourEvent::RequestResponse(_) => {},

                                // handled at the behaviour level
                                BehaviourEvent::Ping { .. }
                                | BehaviourEvent::Identify { .. }
                                | BehaviourEvent::Discovery { .. } => {},
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
                                sender.send(peers).unwrap();
                            }
                            UrsaCommand::GossipsubMessage { topic, message } => {
                                if let Err(error) = swarm.get_mut().behaviour_mut().publish(topic.clone(), message.clone()) {
                                    warn!(
                                        "Failed to publish message top topic {:?} with error {:?}:",
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
    use std::{thread, time::Duration, vec};

    use super::*;

    use db::rocks::RocksDb;
    use simple_logger::SimpleLogger;
    use store::Store;

    fn network_init(config: &UrsaConfig) -> UrsaService<RocksDb> {
        SimpleLogger::new().with_utc_timestamps().init().unwrap();
        let db = RocksDb::open("test_db").expect("Opening RocksDB must succeed");
        let db = Arc::new(db);

        let store = Arc::new(Store::new(Arc::clone(&db)));
        UrsaService::new(&config, Arc::clone(&store))
    }

    // Network Starts
    #[test]
    fn test_network_star() {
        let service = network_init(&UrsaConfig::default());

        task::spawn(async {
            service.start().await;
        });
    }

    // #[async_std::test]
    #[async_std::test]
    async fn test_network_bootstraps() {
        SimpleLogger::new().with_utc_timestamps().init().unwrap();
        let topic = Topic::new(URSA_GLOBAL);

        let db = RocksDb::open("test_db").expect("Opening RocksDB must succeed");
        let db = Arc::new(db);

        let store = Arc::new(Store::new(Arc::clone(&db)));
        let service_1 = UrsaService::new(&UrsaConfig::default(), Arc::clone(&store));

        let mut service_2_config = UrsaConfig::default();
        service_2_config.listen = "/ip4/0.0.0.0/tcp/6010".parse().unwrap();

        let service_2 = UrsaService::new(&service_2_config, Arc::clone(&store));

        let service_1_sender = service_1.command_sender.clone();
        let service_2_receiver = service_2.event_receiver.clone();

        task::spawn(async {
            service_1.start().await;
        });

        task::spawn(async {
            service_2.start().await;
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
        service_1_sender.send(msg).await.unwrap();

        let mut command_receiver = service_2_receiver.fuse();

        loop {
            if let Some(event) = command_receiver.next().await {
                if let UrsaEvent::GossipsubMessage(gossip) = event {
                    print!("{:?}", gossip);
                    break;
                }
            }
        }
    }
}
