use crate::node::Node;
use cid::multihash::Code;
use cid::Cid;
use db::MemoryDB;
use ipld_traversal::blockstore::Blockstore;
use libipld::{cbor::DagCborCodec, ipld, Block, DefaultParams};
use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;
use testground::client::Client;
use tokio::{sync::oneshot, time::timeout};
use ursa_network::NetworkCommand;
use ursa_store::GraphSyncStorage;

fn create_block(content: &[u8]) -> Block<DefaultParams> {
    Block::encode(DagCborCodec, Code::Blake3_256, &ipld!(content)).unwrap()
}

pub async fn run_test(client: &mut Client, node: Node) -> Result<(), String> {
    // Let's wait until all nodes are ready to begin.
    let num_nodes = client.run_parameters().test_instance_count - 1;
    client
        .signal_and_wait("pull-test-ready", num_nodes)
        .await
        .unwrap();

    let block = create_block(&b"hello world"[..]);
    // Pick random node for now.
    let seq = client.global_seq();
    let result = if seq == 2 {
        // Send PUT request and trigger CacheRequest.
        let mut store = GraphSyncStorage(node.store);
        store.insert(&block).unwrap();
        let (sender, receiver) = oneshot::channel();
        let request = NetworkCommand::Put {
            cid: *block.cid(),
            sender,
        };
        node.command_sender.send(request).unwrap();
        receiver.await.unwrap().unwrap();
        Ok(())
    } else {
        // Poll store to see if data has been transferred.
        let check = PendingPullRequest {
            cid: *block.cid(),
            store: GraphSyncStorage(node.store.clone()),
        };
        timeout(Duration::from_secs(5), check)
            .await
            .map_err(|_| "Data transfer failed".to_string())
    };
    // Let's wait until everyone is done.
    client
        .signal_and_wait("test_cache_request_done", num_nodes)
        .await
        .unwrap();
    if result.is_ok() && seq != 2 {
        client.record_message("Data was pulled succesfully")
    }
    result
}

struct PendingPullRequest {
    cid: Cid,
    store: GraphSyncStorage<MemoryDB>,
}

impl Future for PendingPullRequest {
    type Output = ();
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        match self.store.has(&self.cid) {
            Ok(true) => Poll::Ready(()),
            _ => Poll::Pending,
        }
    }
}
