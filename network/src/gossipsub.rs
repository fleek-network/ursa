use crate::config::UrsaConfig;
use anyhow::anyhow;
use std::time::Duration;

use libp2p::gossipsub::{
    Gossipsub, GossipsubConfigBuilder, GossipsubMessage, MessageAuthenticity, MessageId,
    PeerScoreParams, PeerScoreThresholds, ValidationMode,
};

#[derive(Debug)]
pub struct UrsaGossipsub;

impl UrsaGossipsub {
    pub fn new(config: UrsaConfig) -> Self {
        let history_length = 5;
        let history_gossip = 3;
        let mesh_n = 8;
        let mesh_n_low = 4;
        let mesh_n_high = 12;
        let gossip_lazy = mesh_n;
        let heartbeat_interval = Duration::from_secs(1);
        let fanout_ttl = Duration::from_secs(60);
        // D_out
        let mesh_outbound_min = (mesh_n / 2) - 1;
        let max_transmit_size = 1;
        let max_msgs_per_rpc = 1;
        let cache_size = 1;
        let id_fn = move |message: &GossipsubMessage| MessageId::from(todo!());

        let gossip_config = GossipsubConfigBuilder::default()
            .history_length(history_length)
            .history_gossip(history_gossip)
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
            .message_id_fn(id_fn)
            .allow_self_origin(true)
            .mesh_outbound_min(mesh_outbound_min)
            .max_messages_per_rpc(max_msgs_per_rpc)
            .build()
            .expect("gossipsub config");

        let mut gossipsub = Gossipsub::new(MessageAuthenticity::Signed(config.key), gossip_config)
            .map_err(|err| anyhow!("{}", err));

        // Defaults for now
        let params = PeerScoreParams::default();
        let threshold = PeerScoreThresholds::defaults();

        gossipsub.with_peer_score(params, threshold).unwrap()
    }
}
