use ipld_traversal::blockstore::Blockstore as GSBlockstore;
use libipld::{multihash::Code, cbor::DagCborCodec, ipld, Block, DefaultParams};
use libp2p_bitswap::BitswapStore;
use std::time::Duration;
use testground::client::Client;
use tokio::{sync::oneshot, time::{Instant, timeout}};
use ursa_network::NetworkCommand;
use ursa_store::{BitswapStorage, GraphSyncStorage};
use db::Store;

pub mod bootstrap;
pub mod node;
use node::Node;

fn create_block(content: &[u8]) -> Block<DefaultParams> {
    Block::encode(DagCborCodec, Code::Blake3_256, &ipld!(content)).unwrap()
}

pub async fn run_test(client: &mut Client, node: Node) -> Result<(String, Duration), String> {
    // Let's wait until all nodes are ready to begin.
    let num_nodes = client.run_parameters().test_instance_count - 1;

    let seq = client.global_seq();
    if let Err(_) = timeout(Duration::from_secs(60), client.signal_and_wait("fetching-test-ready", num_nodes)).await {
        return Err(format!("[Pull] timeout at `fetching-test-ready` barrier (seq={}).", seq));
    }

    let block = create_block(&b"some data"[..]);
    let (info, duration) = if seq == 2 {
        // Send PUT request .
        let mut store = GraphSyncStorage(node.store.clone());
        store.insert(&block).unwrap();
        let (sender, receiver) = oneshot::channel();
        let request = NetworkCommand::Put {
            cid: *block.cid(),
            sender,
        };
        node.command_sender.send(request).expect("Sending PUT request failed.");
        receiver.await.unwrap().expect("Receiving PUT response failed.");

        // Wait for the data transfer to succeed before removing the block from the store
        if let Err(_) = timeout(Duration::from_secs(60), client.barrier("data-transfer-done", num_nodes - 1)).await {
            return Err(format!("[Pull] timeout at `data-transfer-done` barrier (seq={}).", seq));
        }

        let mut bitswap_store = BitswapStorage(node.store.clone());
        // Remove the block from the store so that it has to be retrieved from
        // the peers for the bitswap get request
        node.store.db.delete(block.cid().to_bytes().as_slice()).unwrap();

        // Send GetBitswap request
        let start = Instant::now();
        let (sender, receiver) = oneshot::channel();
        let request = NetworkCommand::GetBitswap {
            cid: *block.cid(),
            sender,
        };
        node.command_sender.send(request).expect("Sending GetBitswap request failed.");
        receiver.await.unwrap().expect("Receiving GetBitswap response failed.");
        let duration = Instant::now().duration_since(start);

        // Make sure that bitswap get request was successful
        if !bitswap_store.contains(block.cid()).unwrap() {
            client.signal_and_wait("fetching-test-done", num_nodes).await.unwrap();
            return Err("GetBitswap failed, block is not contained in store.".to_string());
        }
        (format!("[Fetching] Results for node {seq}"), duration)
    } else {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        tokio::task::spawn(async move {
            let store = GraphSyncStorage(node.store.clone());
            let cid = *block.cid();
            let start = Instant::now();
            loop {
                if let Ok(true) = store.has(&cid) {
                    sender.send(Ok(())).unwrap();
                    break;
                }
                let duration = Instant::now().duration_since(start);
                if duration > Duration::from_secs(5) {
                    sender
                        .send(Err("Data transfer failed".to_string()))
                        .unwrap();
                    break;
                }
            }
        });
        match receiver.await.unwrap() {
            Ok(()) => {
                client
                    .signal("data-transfer-done")
                    .await
                    .unwrap();
            }
            Err(e) => {
                if let Err(_) = timeout(Duration::from_secs(60), client.signal_and_wait("fetching-test-done", num_nodes)).await {
                    return Err(format!("[Pull] timeout at `data-transfer-done` barrier (seq={}).", seq));
                }
                return Err(e);
            }
        }
        // dummy duration
        (format!("[Fetching] Results for node {seq} [dummy]"), Duration::from_secs(0))
    };

    // Wait until everyone is done.
    if let Err(_) = timeout(Duration::from_secs(60), client.signal_and_wait("fetching-test-done", num_nodes)).await {
        return Err(format!("[Pull] timeout at `data-transfer-done` barrier (seq={}).", seq));
    }

    Ok((info, duration))
}

