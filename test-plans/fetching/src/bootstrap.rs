use db::MemoryDB;
use futures::StreamExt;
use libp2p::PeerId;
use rand::Rng;
use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::Arc;
use testground::client::Client;
use ursa_network::{NetworkCommand, NetworkConfig, UrsaService};
use ursa_store::UrsaStore;

use crate::node::Node;

static NODES_PER_BATCH: u64 = 5;

fn node_init(
    keypair: libp2p::identity::Keypair,
    config: NetworkConfig,
) -> (UrsaService<MemoryDB>, Arc<UrsaStore<MemoryDB>>) {
    let db = Arc::new(MemoryDB::default());
    let store = Arc::new(UrsaStore::new(Arc::clone(&db)));
    (
        UrsaService::new(keypair, &config, Arc::clone(&store)).unwrap(),
        store,
    )
}

pub async fn start_bootstrap(client: testground::client::Client) {
    let local_key = libp2p::identity::Keypair::generate_ed25519();

    let mut config = NetworkConfig::default();
    config.bootstrapper = true;
    config.bootstrap_nodes = vec![];

    let swarm_addr = match if_addrs::get_if_addrs()
        .unwrap()
        .into_iter()
        .find(|iface| iface.name == "eth1")
        .unwrap()
        .addr
        .ip()
    {
        std::net::IpAddr::V4(addr) => format!("/ip4/{addr}/tcp/6009"),
        std::net::IpAddr::V6(_) => unimplemented!(),
    };
    config.swarm_addrs = vec![swarm_addr.clone().parse().unwrap()];

    // Publish its address so other nodes know who is the bootstrapper.
    let peer_id = PeerId::from(local_key.public()).to_string();
    let addr = format!("{}/p2p/{}", swarm_addr, peer_id);
    let payload = serde_json::json!({
        "Addrs": addr,
    });
    client
        .publish("bootstrap-addr", Cow::Owned(payload))
        .await
        .unwrap();

    // Start service.
    let (service, _) = node_init(local_key, config);
    tokio::task::spawn(async move { service.start().await.unwrap() });
    client.record_message(format!("[Bootstrap]: listening at {peer_id:?}"));

    // Let others know that bootstrap is up and running.
    client.signal("bootstrap-ready").await.unwrap();

    // Wait for others to finish before exiting.
    client
        .barrier("done", client.run_parameters().test_instance_count - 1)
        .await
        .unwrap();

    client.record_success().await.expect("Success");
}

pub async fn start_node(client: &mut Client) -> Result<Node, String> {
    let seq = client.global_seq();
    let test_instance_count = client.run_parameters().test_instance_count as usize;
    let num_nodes = client.run_parameters().test_instance_count - 1;

    let bootstrap_addr = client
        .subscribe("bootstrap-addr", test_instance_count)
        .await
        .take(test_instance_count)
        .map(|a| {
            let value = a.unwrap();
            value["Addrs"].as_str().unwrap().to_string()
        })
        .next()
        .await
        .unwrap();
    let local_addr = match if_addrs::get_if_addrs()
        .unwrap()
        .into_iter()
        .find(|iface| iface.name == "eth1")
        .unwrap()
        .addr
        .ip()
    {
        std::net::IpAddr::V4(addr) => format!("/ip4/{addr}/tcp/0"),
        std::net::IpAddr::V6(_) => unimplemented!(),
    };

    let mut config = NetworkConfig::default();
    config.bootstrap_nodes = vec![bootstrap_addr.parse().unwrap()];
    config.swarm_addrs = vec![local_addr.parse().unwrap()];

    let mut batch_index = 0;
    // Divide the nodes in batches of size `NODES_PER_BATCH`.
    // The last batch may have less than `NODES_PER_BATCH` nodes.
    let num_batches = ((num_nodes as f64) / (NODES_PER_BATCH as f64)).ceil() as u64;
    // Start iteration at 1 so that the nodes in the first batch don't run into the barrier.
    // If `num_nodes` is less than `NODES_PER_BATCH`, `num_batches` will be 1 and the loop
    // is never entered, so nodes don't run into a barrier in that case
    for i in 1..num_batches {
        let lower = i * NODES_PER_BATCH + 1;
        let upper = if i == num_batches - 1 {
            num_nodes
        } else {
            i * NODES_PER_BATCH + NODES_PER_BATCH
        };
        if seq >= lower && seq <= upper {
            batch_index = i;

            // Wait for previous batch to finish
            client
                .barrier(format!("batch-{}-done", batch_index - 1), NODES_PER_BATCH)
                .await
                .unwrap();
        }
    }

    // Wait for bootstrapping node.
    client.barrier("bootstrap-ready", 1).await.unwrap();

    // Each peer should wait before attempting to bootstrap.
    // If all peers in the batch attempt to bootstrap at the same time, DHT won't
    // be updated in time for peers to discover each other.
    // We have no way to trigger bootstrapping later.
    let mut rng = rand::thread_rng();
    let interval = rng.gen_range(1..NODES_PER_BATCH * 4);
    tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
    client.record_message(format!(
        "[Node]: Bootstrapping to address {}",
        bootstrap_addr
    ));

    // Init service.
    let local_key = libp2p::identity::Keypair::generate_ed25519();
    let (service, store) = node_init(local_key.clone(), config);
    let cmd_sender = service.command_sender();

    // Start service.
    tokio::task::spawn(async move { service.start().await.unwrap() });
    let peer_count = test_instance_count - 1;

    // Send a command to get the service's peers.
    let mut peers = HashSet::new();
    for _ in 0..3 {
        if peers.len() == peer_count {
            break;
        }
        // Give discovery some time.
        client.record_message("[Node]: Wait for discovery");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let (peers_sender, peers_receiver) = tokio::sync::oneshot::channel();
        let msg = NetworkCommand::GetPeers {
            sender: peers_sender,
        };
        cmd_sender.send(msg).unwrap();
        peers = peers_receiver.await.expect("Success");

        client.record_message(format!("[Node]: Node {} has {} peers", seq, peers.len()));
    }

    // Once all nodes in a batch have reached this point, the nodes in the next batch
    // are notified and can continue.
    client
        .signal(format!("batch-{}-done", batch_index))
        .await
        .unwrap();

    if peers.len() > 1 {
        Ok(Node::new(store, cmd_sender))
    } else {
        Err("failed to bootstrap".to_string())
    }
}
