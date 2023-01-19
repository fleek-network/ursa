use cid::multihash::Code;
use env_logger::Env;
use ipld_traversal::blockstore::Blockstore;
use libipld::{cbor::DagCborCodec, ipld, Block, DefaultParams};
use std::time::Duration;
use testground::client::Client;
use tokio::{sync::oneshot, time::Instant};
use ursa_network::NetworkCommand;
use ursa_store::GraphSyncStorage;

use data_transfer::{bootstrap, node::Node};

#[tokio::main]
async fn main() {
    let mut client = Client::new_and_init().await.unwrap();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // The first instance that arrives will be the bootstrapper.
    let seq = client.global_seq();
    if seq == 1 {
        return bootstrap::start_bootstrap(client).await;
    }

    let num_nodes = client.run_parameters().test_instance_count - 1;

    let node = match bootstrap::start_node(&mut client).await {
        Ok(node) => node,
        Err(e) => {
            // All nodes wait here and signal to the bootstrap node that they are done.
            client.signal_and_wait("done", num_nodes).await.unwrap();
            client.record_failure(e).await.expect("Success");
            return;
        }
    };

    let result = run_test(&mut client, node).await;

    // All nodes wait here and signal to the bootstrap node that they are done.
    client.signal_and_wait("done", num_nodes).await.unwrap();

    match result {
        Ok((test_name, duration)) => {
            client.record_message(format!("{test_name}: {duration:?}"));
            client.record_success().await.expect("Success")
        }
        Err(e) => client.record_failure(e).await.expect("Success"),
    }
}

fn create_block(content: &[u8]) -> Block<DefaultParams> {
    Block::encode(DagCborCodec, Code::Blake3_256, &ipld!(content)).unwrap()
}

pub async fn run_test(client: &mut Client, node: Node) -> Result<(String, Duration), String> {
    // Let's wait until all nodes are ready to begin.
    let num_nodes = client.run_parameters().test_instance_count - 1;
    client
        .signal_and_wait("pull-test-ready", num_nodes)
        .await
        .unwrap();

    let block = create_block(&b"hello world"[..]);
    // Pick random node for now.
    let seq = client.global_seq();
    let start = Instant::now();
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
        receiver.await.unwrap()
    };
    let duration = Instant::now().duration_since(start);
    // Let's wait until everyone is done.
    client
        .signal_and_wait("pull-test-done", num_nodes)
        .await
        .unwrap();

    result.map(|_| (format!("[Pull] Results for node {seq}"), duration))
}
