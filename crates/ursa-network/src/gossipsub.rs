use crate::config::NetworkConfig;
use anyhow::anyhow;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use libp2p::{
    gossipsub::{
        Gossipsub, GossipsubConfigBuilder, GossipsubMessage, MessageAuthenticity, MessageId,
        ValidationMode,
    },
    identity::Keypair,
};

const URSA_GOSSIP_PROTOCOL: &str = "ursa/gossipsub/0.0.1";

///
#[derive(Debug)]
pub struct UrsaGossipsub;

impl UrsaGossipsub {
    pub fn new(keypair: &Keypair, config: &NetworkConfig) -> Gossipsub {
        let is_bootstrapper = config.bootstrapper;
        let mesh_n = if is_bootstrapper { 0 } else { 8 };
        let mesh_n_low = if is_bootstrapper { 0 } else { 4 };
        let mesh_n_high = if is_bootstrapper { 0 } else { 12 };
        let gossip_lazy = mesh_n;
        // D_out
        let mesh_outbound_min = if is_bootstrapper { 0 } else { (mesh_n / 2) - 1 };
        let max_transmit_size = 4 * 1024 * 1024;
        let message_id_fn = move |message: &GossipsubMessage| {
            let mut hasher = DefaultHasher::new();
            message.data.hash(&mut hasher);
            MessageId::from(hasher.finish().to_string())
        };

        let gossip_config = GossipsubConfigBuilder::default()
            .protocol_id_prefix(URSA_GOSSIP_PROTOCOL)
            .mesh_n(mesh_n)
            .mesh_n_low(mesh_n_low)
            .mesh_n_high(mesh_n_high)
            // default to mesh_n
            .gossip_lazy(gossip_lazy)
            .max_transmit_size(max_transmit_size)
            .validation_mode(ValidationMode::Strict)
            .message_id_fn(message_id_fn)
            .mesh_outbound_min(mesh_outbound_min)
            .build()
            .expect("gossipsub config");

        Gossipsub::new(MessageAuthenticity::Signed(keypair.clone()), gossip_config)
            .map_err(|err| anyhow!("{}", err))
            .unwrap()
    }
}
