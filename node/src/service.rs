//! Fnet Service implementation.
//!
//!
//!

use async_std::task;
use futures::select;
use libipld::store::StoreParams;
use libp2p::{
    gossipsub::IdentTopic as Topic,
    identity::Keypair,
    swarm::{ConnectionLimits, SwarmBuilder, SwarmEvent},
    PeerId, Swarm,
};
use libp2p_bitswap::BitswapStore;
use tracing::{trace, warn};

use crate::{behaviour::FnetBehaviour, config::FnetConfig, transport::FnetTransport};

pub const PROTOCOL_NAME: &[u8] = b"/fnet/0.0.1";

const NETWORK_IDENTITY: &'static str = "fleek-network";
const NETWORK_PROTOCOL: &'static str = "/fnet/0.0.1";

pub struct FnetService<P: StoreParams> {
    swarm: Swarm<FnetBehaviour<P>>,
}

impl<P: StoreParams> FnetService<P> {
    /// Init a new [`FnetService`] based on [`FnetConfig`]
    ///
    /// For fnet [identity] we use ed25519 either
    /// checking for a local store or creating a new keypair.
    ///
    /// For fnet [transport] we build a default QUIC layer and
    /// failover to tcp.
    ///
    /// For fnet behaviour we use [`FnetBehaviour`].
    ///
    /// We construct a [`Swarm`] with [`FnetTransport`] and [`FnetBehaviour`]
    /// listening on [`FnetConfig`] `swarm_addr`.
    ///
    ///
    pub fn new<S: BitswapStore<Params = P>>(config: &FnetConfig, store: S) -> Self {
        // Todo: Create or get from local store
        let keypair = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(keypair.public());

        let transport = FnetTransport::new(&mut config).build();

        let behaviour = FnetBehaviour::new(&mut config, store);

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
            .executor(Box::new(|f| {
                task::spawn(f);
            }))
            .build();

        match Swarm::listen_on(&mut swarm, config.swarm_addr) {
            Ok(listener_id) => todo!(),
            Err(error) => todo!(),
        };

        // subscribe to topic
        let topic = Topic::new(todo!());
        if let Err(error) = swarm.behaviour_mut().subscribe(&topic) {
            warn!("Failed to subscribe with topic: {}", error);
        }

        // boostrap
        if let Err(error) = swarm.behaviour_mut().bootstrap() {
            warn!("Failed to bootstrap with Kademlia: {}", error);
        }

        FnetService { swarm }
    }

    /// Start the ursa network service
    pub async fn start(&self) {
        let swarm = self.swarm;

        loop {
            select! {
                event = swarm.select_next_some() => match event {
                    // _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {}
