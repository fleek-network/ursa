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

use anyhow::{Error, Result};
use async_std::{
    channel::{unbounded, Receiver, Sender},
    prelude::StreamExt,
    task,
};
use futures::{channel::oneshot, select};
use libipld::store::StoreParams;
use libp2p::{
    core::either::EitherError,
    gossipsub::{GossipsubEvent, GossipsubMessage, IdentTopic as Topic},
    identity::Keypair,
    request_response::RequestResponseEvent,
    swarm::{ConnectionHandlerUpgrErr, ConnectionLimits, SwarmBuilder, SwarmEvent},
    PeerId, Swarm,
};
use libp2p_bitswap::{BitswapEvent, BitswapStore};
use std::collections::HashSet;
use tiny_cid::Cid;
use tracing::{info, warn};

use crate::{
    behaviour::{Behaviour, BehaviourEvent},
    codec::protocol::{UrsaExchangeRequest, UrsaExchangeResponse},
    config::UrsaConfig,
    transport::UrsaTransport,
};

pub const URSA_GLOBAL: &str = "/ursa/global";
pub const MESSAGE_PROTOCOL: &[u8] = b"/ursa/message/0.0.1";

#[derive(Debug)]
struct GetProviders {
    cid: Cid,
    sender: oneshot::Sender<HashSet<PeerId>>,
}

#[derive(Debug)]
struct StartProviding {
    cid: Cid,
    sender: oneshot::Sender<Result<()>>,
}

#[derive(Debug)]
struct GossipsubMessageCommand;

#[derive(Debug)]
pub enum UrsaCommand {
    GetProviders(GetProviders),
    StartProviding(StartProviding),
    GossipsubMessage(GossipsubMessageCommand),
}

#[derive(Debug)]
pub enum UrsaEvent {
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    BitswapEvent(BitswapEvent),
    GossipsubMessage(GossipsubMessage),
}

pub struct UrsaService<P: StoreParams> {
    /// The main libp2p swamr emitting events.
    swarm: Swarm<Behaviour<P>>,
    /// Handles outbound messages to peers
    command_sender: Sender<UrsaCommand>,
    /// Handles inbound messages from peers
    command_receiver: Receiver<UrsaCommand>,
    /// Handles events emitted by the ursa network
    event_sender: Sender<UrsaEvent>,
    /// Handles events received by the ursa network
    event_receiver: Receiver<UrsaEvent>,
}

impl<P: StoreParams> UrsaService<P> {
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
    pub fn new<S: BitswapStore<Params = P>>(config: &UrsaConfig, store: S) -> Result<Self> {
        // Todo: Create or get from local store
        let keypair = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(keypair.public());

        info!(target: "ursa-libp2p", "Node identity is: {}", local_peer_id.to_base58());

        let transport = UrsaTransport::new(&keypair, &mut config);

        let behaviour = Behaviour::new(&keypair, &mut config, store);

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
            command_sender,
            command_receiver,
            event_sender,
            event_receiver,
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
                event = swarm.next() => match event {
                    Some(event) => {
                        if let Err(err) = self.handle_event(event).await {
                            warn!("Swarm Event: {:?}", err);
                        }
                    },
                    None => return,
                },
                command = command_receiver.next() => match command {
                    Some(command) => {
                        if let Err(err) = self.handle_command(command).await {
                            warn!("Swarm Command: {:?}", err);
                        }
                    },
                    None => return,
                },
            }
        }
    }

    fn handle_bitswap(&self, event: BitswapEvent) {
        todo!()
    }

    fn handle_gossipsub(&self, event: GossipsubEvent) {
        todo!()
    }

    fn handle_request_response(
        &self,
        event: RequestResponseEvent<UrsaExchangeRequest, UrsaExchangeResponse>,
    ) {
        todo!()
    }

    async fn handle_event(
        &mut self,
        event: SwarmEvent<BehaviourEvent, EitherError<ConnectionHandlerUpgrErr<Error>, Error>>,
    ) {
        match event {
            SwarmEvent::Behaviour(event) => match event {
                BehaviourEvent::Bitswap(event) => self.handle_bitswap(event),
                BehaviourEvent::Gossip(event) => self.handle_gossipsub(event),
                BehaviourEvent::RequestResponse(event) => self.handle_request_response(event),

                // handled at the behaviour level
                BehaviourEvent::Ping { .. }
                | BehaviourEvent::Identify { .. }
                | BehaviourEvent::Discovery { .. } => {}
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
            | SwarmEvent::OutgoingConnectionError { .. } => {}
        }
    }

    async fn handle_command(&mut self, command: UrsaCommand) {
        match command {
            UrsaCommand::GetProviders(_) => todo!(),
            UrsaCommand::StartProviding(_) => todo!(),
            UrsaCommand::GossipsubMessage(_) => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use libipld::store::StoreParams;

    use super::UrsaService;

    fn ursa_service<P: StoreParams>() -> UrsaService<P> {
        todo!()
    }

    // Network Starts
    #[test]
    fn ursa_service_start() {
        todo!()
    }
}
