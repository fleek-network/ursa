use db::MemoryDB;
use tokio::sync::oneshot::Sender;
use ursa_network::NetworkCommand;
use ursa_store::GraphSyncStorage;

pub struct Node {
    pub store: GraphSyncStorage<MemoryDB>,
    pub command_sender: Sender<NetworkCommand>,
}
