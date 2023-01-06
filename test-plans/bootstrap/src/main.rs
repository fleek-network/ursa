use db::MemoryDB;
use env_logger::Env;
use futures::StreamExt;
use libp2p::{Multiaddr, PeerId};
use std::borrow::Cow;
use std::sync::Arc;
use std::collections::HashSet;
use ursa_index_provider::config::ProviderConfig;
use ursa_index_provider::provider::Provider;
use ursa_network::{NetworkCommand, NetworkConfig, UrsaService};
use ursa_store::UrsaStore;

#[tokio::main]
async fn main() {
    let client = testground::client::Client::new_and_init().await.unwrap();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // The first instance that arrives will be the bootstrapper.
    let seq = client.global_seq();
    if seq == 1 {
        return run_bootstrap(client).await;
    }

    let test_instance_count = client.run_parameters().test_instance_count as usize;

    let mut bootstrap_peer_id = client
        .subscribe("bootstrap-addr", test_instance_count)
        .await
        .take(test_instance_count)
        .map(|a| {
            let value = a.unwrap();
            value["Addrs"].as_str().unwrap().to_string()
        });

    let bootstrap_addr = bootstrap_peer_id.next().await.unwrap();

    client.record_message(format!("Node: Bootstrapping to address {}", bootstrap_addr));

    let mut config = NetworkConfig::default();
    config.bootstrap_nodes = vec![bootstrap_addr.parse().unwrap()];
    config.swarm_addrs = vec![format!("/ip4/0.0.0.0/tcp/600{}", seq).parse().unwrap()];

    // Wait until bootstrapping is done.
    client.barrier("bootstrap-ready", 1).await.unwrap();

    // Start service.
    let local_key = libp2p::identity::Keypair::generate_ed25519();
    let service = node_init(local_key.clone(), config);

    let cmd_sender = service.command_sender();
    tokio::task::spawn(async move { service.start().await.unwrap() });

    // Send a command to get the service's peers.
    let mut peers = HashSet::new();
    while peers.len() < 1 {
        // Give discovery some time.
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let (peers_sender, peers_receiver) = tokio::sync::oneshot::channel();
        let msg = NetworkCommand::GetPeers {
            sender: peers_sender,
        };
        cmd_sender.send(msg).unwrap();

        peers = peers_receiver.await.expect("Success");

        client.record_message(format!("Node: Peers: {:?}", peers));
    }

    if peers.contains(
        &PeerId::try_from_multiaddr(&Multiaddr::try_from(bootstrap_addr).unwrap()).unwrap(),
    ) {
        client.signal("done").await.unwrap();
        client.record_success().await.expect("Success");
    }
}

fn node_init(keypair: libp2p::identity::Keypair, config: NetworkConfig) -> UrsaService<MemoryDB> {
    let db = Arc::new(MemoryDB::default());
    let store = Arc::new(UrsaStore::new(Arc::clone(&db)));
    UrsaService::new(keypair, &config, Arc::clone(&store)).unwrap()
}

async fn run_bootstrap(client: testground::client::Client) {
    let local_key = libp2p::identity::Keypair::generate_ed25519();

    let mut config = NetworkConfig::default();
    config.bootstrapper = true;
    config.bootstrap_nodes = vec![];

    let swarm_addr = "/ip4/0.0.0.0/tcp/6009";
    config.swarm_addrs = vec![swarm_addr.clone().parse().unwrap()];

    let addr = format!("{}/p2p/{}", swarm_addr, PeerId::from(local_key.public()));
    // Publish its address so other nodes know who is the bootstrapper.
    let payload = serde_json::json!({
        "Addrs": addr,
    });
    client
        .publish("bootstrap-addr", Cow::Owned(payload))
        .await
        .unwrap();

    // Start service.
    let service = node_init(local_key, config);

    tokio::task::spawn(async move { service.start().await.unwrap() });

    client.record_message(format!("Bootstrap: listening at {}", addr));

    // Let others know that bootstrap is up and running.
    client.signal("bootstrap-ready").await.unwrap();

    // Wait for others to finish before exiting.
    client
        .barrier("done", client.run_parameters().test_instance_count - 1)
        .await
        .unwrap();

    client.record_success().await.expect("Success");
}
