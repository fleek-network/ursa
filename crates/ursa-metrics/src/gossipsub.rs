use libp2p_gossipsub::GossipsubEvent;
use metrics::increment_counter;
use metrics::Label;

impl super::Recorder for GossipsubEvent {
    fn record(&self) {
        match self {
            GossipsubEvent::Message { message, .. } => {
                increment_counter!(
                    "gossipsub_message_received",
                    vec![Label::new("topic", message.topic.to_string()),]
                );
            }
            GossipsubEvent::GossipsubNotSupported { peer_id } => {
                increment_counter!(
                    "gossipsub_peer_not_supported",
                    vec![Label::new("peer", peer_id.to_string()),]
                );
            }
            GossipsubEvent::Subscribed { .. } => {}
            GossipsubEvent::Unsubscribed { .. } => {}
        }
    }
}
