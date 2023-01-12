use db::MemoryDB;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender as Sender;
use ursa_network::NetworkCommand;
use ursa_store::UrsaStore;

pub struct Node {
    pub store: Arc<UrsaStore<MemoryDB>>,
    pub command_sender: Sender<NetworkCommand>,
}

impl Node {
    pub fn new(store: Arc<UrsaStore<MemoryDB>>, command_sender: Sender<NetworkCommand>) -> Self {
        Self {
            store,
            command_sender,
        }
    }
}
