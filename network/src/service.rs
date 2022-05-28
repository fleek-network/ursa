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

use async_std::{
    channel::{unbounded, Receiver, Sender},
    prelude::StreamExt,
    task,
};
use futures::{select, FutureExt};
use libipld::store::StoreParams;
use libp2p::{
    core::either::EitherError,
    gossipsub::IdentTopic as Topic,
    identity::Keypair,
    swarm::{ConnectionHandlerUpgrErr, ConnectionLimits, SwarmBuilder, SwarmEvent},
    PeerId, Swarm,
};
use libp2p_bitswap::BitswapStore;
use tracing::{info, warn};

use crate::{
    behaviour::{Behaviour, BehaviourEvent},
    config::UrsaConfig,
    transport::UrsaTransport,
};

pub const PROTOCOL_NAME: &[u8] = b"/ursa/0.0.1";
pub const MESSAGE_PROTOCOL: &[u8] = b"/ursa/message/0.0.1";

#[derive(Debug)]
pub enum UrsaCommand {}

#[derive(Debug)]
pub enum UrsaEvent {}

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
    pub fn new<S: BitswapStore<Params = P>>(config: &UrsaConfig, store: S) -> Self {
        // Todo: Create or get from local store
        let keypair = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(keypair.public());

        info!(target: "ursa-libp2p", "Node identity is: {}", local_peer_id.to_base58());

        let transport = UrsaTransport::new(&mut config).build();

        let behaviour = Behaviour::new(&mut config, store);

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

        Swarm::listen_on(&mut swarm, config.swarm_addr)
            .unwrap()
            .expect("swarm can be started");

        // subscribe to topic
        let topic = Topic::new(todo!());
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
        loop {
            select! {
                event = self.swarm.next() => self.handle_event(event).await,
                command = self.command_receiver.next() => match command {
                    Some(command) => self.handle_command(command).await,
                    None => return,
                },
            }
        }
    }

    async fn handle_event(
        &mut self,
        event: SwarmEvent<
            BehaviourEvent,
            EitherError<ConnectionHandlerUpgrErr<anyhow::Error>, anyhow::Error>,
        >,
    ) {
        match event {
            SwarmEvent::Behaviour(event) => match event {
                BehaviourEvent::Bitswap(event) => self.swarm.behaviour_mut().bitswap_handler(event),
                BehaviourEvent::Gossip(event) => {
                    self.swarm.behaviour_mut().gossipsub_handler(event)
                }

                // All the events are already handled in [Behaviour]
                // maybe we should exclude them from [BehaviourEvent]
                _ => {}
            },

            // Do we need to handle any of the below events?
            SwarmEvent::ConnectionEstablished {
                peer_id,
                endpoint,
                num_established,
                concurrent_dial_errors,
            } => todo!(),
            SwarmEvent::ConnectionClosed {
                peer_id,
                endpoint,
                num_established,
                cause,
            } => todo!(),
            SwarmEvent::IncomingConnection {
                local_addr,
                send_back_addr,
            } => todo!(),
            SwarmEvent::IncomingConnectionError {
                local_addr,
                send_back_addr,
                error,
            } => todo!(),
            SwarmEvent::OutgoingConnectionError { peer_id, error } => todo!(),
            SwarmEvent::BannedPeer { peer_id, endpoint } => todo!(),
            SwarmEvent::NewListenAddr {
                listener_id,
                address,
            } => todo!(),
            SwarmEvent::ExpiredListenAddr {
                listener_id,
                address,
            } => todo!(),
            SwarmEvent::ListenerClosed {
                listener_id,
                addresses,
                reason,
            } => todo!(),
            SwarmEvent::ListenerError { listener_id, error } => todo!(),
            SwarmEvent::Dialing(_) => todo!(),
        }
    }

    async fn handle_command(&mut self, command: UrsaCommand) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::config::UrsaConfig;

    use super::UrsaService;

    async fn ursa_service() -> UrsaService {
        UrsaService::new(&UrsaConfig::default()).await.unwrap()
    }

    // Network Starts
    #[test]
    fn ursa_service_start() {
        todo!()
    }
}
