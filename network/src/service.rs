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

use anyhow::Result;
use async_std::{
    channel::{unbounded, Receiver, Sender},
    task,
};
use futures::{channel::oneshot, select, StreamExt};
use ipld_blockstore::BlockStore;
use libipld::{store::StoreParams, DefaultParams};
use libp2p::{
    gossipsub::{GossipsubEvent, GossipsubMessage, IdentTopic as Topic},
    identity::Keypair,
    request_response::RequestResponseEvent,
    swarm::{ConnectionLimits, SwarmBuilder, SwarmEvent},
    PeerId, Swarm,
};
use libp2p_bitswap::BitswapEvent;
use std::{collections::HashSet, marker::PhantomData, sync::Arc};
use store::{BitswapStorage, Store};
use tiny_cid::Cid;
use tracing::{info, warn};

use crate::{
    behaviour::{Behaviour, BehaviourEvent, BehaviourEventError},
    codec::protocol::{UrsaExchangeRequest, UrsaExchangeResponse},
    config::UrsaConfig,
    transport::UrsaTransport,
};

pub const URSA_GLOBAL: &str = "/ursa/global";
pub const MESSAGE_PROTOCOL: &[u8] = b"/ursa/message/0.0.1";

#[derive(Debug)]
struct Get {
    cid: Cid,
    sender: oneshot::Sender<HashSet<PeerId>>,
}

#[derive(Debug)]
struct Put {
    cid: Cid,
    sender: oneshot::Sender<Result<()>>,
}

#[derive(Debug)]
struct GossipsubMessageCommand;

#[derive(Debug)]
pub enum UrsaCommand {
    /// Rpc commands
    Get(Get),
    Put(Put),

    /// inter-network commands
    GossipsubMessage(GossipsubMessageCommand),
}

#[derive(Debug)]
pub enum UrsaEvent {
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),

    BitswapEvent(BitswapEvent),
    GossipsubMessage(GossipsubMessage),
}

pub struct UrsaService<P: StoreParams, S> {
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
    _marker: PhantomData<P>,
}

impl<P, S> UrsaService<P, S>
where
    P: StoreParams,
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
    pub fn new(config: &UrsaConfig, store: Arc<Store<S>>) -> Result<Self> {
        // Todo: Create or get from local store
        let keypair = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(keypair.public());

        info!(target: "ursa-libp2p", "Node identity is: {}", local_peer_id.to_base58());

        let transport = UrsaTransport::new(&keypair, &mut config);

        let bitswap_store = BitswapStorage(store.clone());

        let behaviour = Behaviour::new(&keypair, &mut config, bitswap_store);

        let limits = ConnectionLimits::default()
            .with_max_pending_incoming(todo!())
            .with_max_pending_outgoing(todo!())
            .with_max_established_incoming(todo!())
            .with_max_established_outgoing(todo!())
            .with_max_established(todo!())
            .with_max_established_per_peer(todo!());

        let mut swarm = SwarmBuilder::new(transport, behaviour, local_peer_id)
            // .notify_handler_buffer_size(todo!())
            // .connection_event_buffer_size(todo!())
            .connection_limits(limits)
            .executor(Box::new(|future| {
                task::spawn(future);
            }))
            .build();

        Swarm::listen_on(&mut swarm, config.swarm_addr).unwrap();

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

        Ok(UrsaService {
            swarm,
            store,
            command_sender,
            command_receiver,
            event_sender,
            event_receiver,
            _marker: PhantomData,
        })
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
                        if let Err(err) = self.handle_event(event).await {
                            warn!("Swarm Event: {:?}", err);
                        }
                    }
                },
                command = command_receiver.next() => {
                    if let Some(command) = command {
                        if let Err(err) = self.handle_command(command).await {
                            warn!("Command Event: {:?}", err);
                        }
                    }
                },
            }
        }
    }

    fn handle_bitswap(&self, event: BitswapEvent) -> Result<()> {
        todo!()
    }

    fn handle_gossipsub(&self, event: GossipsubEvent) -> Result<()> {
        todo!()
    }

    fn handle_request_response(
        &self,
        event: RequestResponseEvent<UrsaExchangeRequest, UrsaExchangeResponse>,
    ) -> Result<()> {
        todo!()
    }

    async fn handle_event(
        &mut self,
        event: SwarmEvent<BehaviourEvent, BehaviourEventError>,
    ) -> Result<()> {
        match event {
            SwarmEvent::Behaviour(event) => match event {
                BehaviourEvent::Bitswap(event) => self.handle_bitswap(event),
                BehaviourEvent::Gossip(event) => self.handle_gossipsub(event),
                BehaviourEvent::RequestResponse(event) => self.handle_request_response(event),

                // handled at the behaviour level
                BehaviourEvent::Ping { .. }
                | BehaviourEvent::Identify { .. }
                | BehaviourEvent::Discovery { .. } => Ok({}),
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
            | SwarmEvent::OutgoingConnectionError { .. } => Ok({}),
        }
    }

    async fn handle_command(&mut self, command: UrsaCommand) -> Result<()> {
        match command {
            UrsaCommand::Get(_) => todo!(),
            UrsaCommand::Put(_) => todo!(),
            UrsaCommand::GossipsubMessage(_) => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use libipld::store::StoreParams;

    use super::UrsaService;

    // Network Starts
    #[test]
    fn ursa_service_start() {
        todo!()
    }
}
