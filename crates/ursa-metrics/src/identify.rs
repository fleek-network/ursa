use lazy_static::lazy_static;
use libp2p_core::PeerId;
use libp2p_identify::IdentifyEvent;
use metrics::{increment_counter, increment_gauge, Label};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

lazy_static! {
    pub static ref PEERS: Arc<RwLock<HashMap<PeerId, Vec<String>>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

impl super::Recorder for IdentifyEvent {
    fn record(&self) {
        match self {
            IdentifyEvent::Received { peer_id, info } => {
                let mut peers = PEERS.write().unwrap();
                if peers.insert(*peer_id, info.protocols.clone()).is_none() {
                    for protocol in &info.protocols {
                        increment_gauge!(
                            "identify_supported_protocols",
                            1.0,
                            vec![Label::new("protocol", protocol.clone())]
                        );
                    }
                }
            }
            IdentifyEvent::Sent { .. } => {
                increment_counter!("identify_sent");
            }
            IdentifyEvent::Error { .. } => {
                increment_counter!("identify_error");
            }
            IdentifyEvent::Pushed { .. } => {
                increment_counter!("identify_pushed");
            }
        }
    }
}
