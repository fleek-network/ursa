//! Ursa Service implementation.
//!
//!
//!

use async_std::{prelude::StreamExt, task};
use futures::select;
use libipld::store::StoreParams;
use libp2p::{
    core::either::EitherError,
    gossipsub::IdentTopic as Topic,
    identity::Keypair,
    swarm::{ConnectionHandlerUpgrErr, ConnectionLimits, SwarmBuilder, SwarmEvent},
    PeerId, Swarm,
};
use libp2p_bitswap::BitswapStore;
use tracing::{trace, warn};

use crate::{
    behaviour::{Behaviour, BehaviourEvent},
    config::UrsaConfig,
    transport::UrsaTransport,
};

pub const PROTOCOL_NAME: &[u8] = b"/ursa/0.0.1";
pub const MESSAGE_PROTOCOL: &[u8] = b"/ursa/message/0.0.1";


pub struct UrsaService<P: StoreParams> {
    swarm: Swarm<Behaviour<P>>,
    /// Handles inbound requests from peers

    /// Handles
    
}

impl<P: StoreParams> UrsaService<P> {
    /// Init a new [`UrsaService`] based on [`UrsaConfig`]
    ///
    /// For ursa [identity] we use ed25519 either
    /// checking for a local store or creating a new keypair.
    ///
    /// For ursa [transport] we build a default QUIC layer and
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

        // boostrap
        if let Err(error) = swarm.behaviour_mut().bootstrap() {
            warn!("Failed to bootstrap with Kademlia: {}", error);
        }

        UrsaService { swarm }
    }

    /// Start the ursa network service
    pub async fn start(mut self) {
        loop {
            select! {
                event = self.swarm.next() => self.handle_event(event).await
            }
        }
    }

    pub async fn handle_event(
        &mut self,
        event: SwarmEvent<
            BehaviourEvent,
            EitherError<ConnectionHandlerUpgrErr<anyhow::Error>, anyhow::Error>,
        >,
    ) {
        match event {
            SwarmEvent::Behaviour(event) => match event {
                BehaviourEvent::Bitswap(_) => todo!(),
                BehaviourEvent::Gossip(_) => todo!(),

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
}

#[cfg(test)]
mod tests {}
