use crate::config::UrsaConfig;
use anyhow::anyhow;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    time::Duration,
};

use libp2p::{
    gossipsub::{
        Gossipsub, GossipsubConfigBuilder, GossipsubMessage, MessageAuthenticity, MessageId,
        ValidationMode,
    },
    identity::Keypair,
};

const BOOTSTRAP_MESH_N: usize = 0;
const BOOTSTRAP_MESH_LOW: usize = 0;
const BOOTSTRAP_MESH_HIGH: usize = 0;
// D out
const BOOTSTRAP_MESH_OUTBOUND_MIN: usize = 0;

const NODE_MESH_N: usize = 8;
const NODE_MESH_LOW: usize = 4;
const NODE_MESH_HIGH: usize = 12;
const NODE_MESH_OUTBOUND_MIN: usize = (NODE_MESH_N / 2) - 1;

const URSA_GOSSIP_PROTOCOL: &str = "ursa/gossipsub/0.0.1";

///
#[derive(Debug)]
pub struct UrsaGossipsub;

impl UrsaGossipsub {
    pub fn new(keypair: &Keypair, config: &UrsaConfig) -> Gossipsub {
        let (mesh_n, mesh_n_low, mesh_n_high, mesh_outbound_min) = if config.bootstrap_mode {
            (
                BOOTSTRAP_MESH_N,
                BOOTSTRAP_MESH_LOW,
                BOOTSTRAP_MESH_HIGH,
                BOOTSTRAP_MESH_OUTBOUND_MIN,
            )
        } else {
            (
                NODE_MESH_N,
                NODE_MESH_LOW,
                NODE_MESH_HIGH,
                NODE_MESH_OUTBOUND_MIN,
            )
        };

        let gossip_lazy = mesh_n;
        let heartbeat_interval = Duration::from_secs(1);
        let fanout_ttl = Duration::from_secs(60);
        let max_transmit_size = 4 * 1024 * 1024;
        // todo(botch): should we limit the number here?
        let max_msgs_per_rpc = 1;
        let cache_size = Duration::from_secs(60);
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
            .gossip_lazy(mesh_n)
            .gossip_lazy(gossip_lazy)
            .heartbeat_interval(heartbeat_interval)
            .fanout_ttl(fanout_ttl)
            .max_transmit_size(max_transmit_size)
            .duplicate_cache_time(cache_size)
            .validate_messages()
            .validation_mode(ValidationMode::Strict)
            .message_id_fn(message_id_fn)
            .allow_self_origin(true)
            .mesh_outbound_min(mesh_outbound_min)
            .max_messages_per_rpc(Some(max_msgs_per_rpc))
            .build()
            .expect("gossipsub config");

        Gossipsub::new(MessageAuthenticity::Signed(keypair.clone()), gossip_config)
            .map_err(|err| anyhow!("{}", err))
            .unwrap()
    }
}
