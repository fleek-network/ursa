use db::MemoryDB;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender as Sender;
use ursa_network::NetworkCommand;
use ursa_store::{GraphSyncStorage, UrsaStore};

pub struct Node {
    pub store: GraphSyncStorage<MemoryDB>,
    pub command_sender: Sender<NetworkCommand>,
}

impl Node {
    pub fn new(store: Arc<UrsaStore<MemoryDB>>, command_sender: Sender<NetworkCommand>) -> Self {
        Self {
            store: GraphSyncStorage(store),
            command_sender,
        }
    }
}
