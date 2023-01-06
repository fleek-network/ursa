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
use testground::client::Client;

#[tokio::main]
async fn main() {
    let mut client = Client::new_and_init().await.unwrap();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // The first instance that arrives will be the bootstrapper.
    let seq = client.global_seq();
    if seq == 1 {
        return run_bootstrap(client).await;
    }

    if let Err(e) = network_init(&mut client).await {
        client.record_failure(e).await.expect("Success");
    } else {
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
    let service = node_init(local_key, config);

    tokio::task::spawn(async move { service.start().await.unwrap() });

    client.record_message(format!("Bootstrap: listening at {:?}", peer_id));

    // Let others know that bootstrap is up and running.
    client.signal("bootstrap-ready").await.unwrap();

    // Wait for others to finish before exiting.
    client
        .barrier("done", client.run_parameters().test_instance_count - 1)
        .await
        .unwrap();

    client.record_success().await.expect("Success");
}

async fn network_init(client: &mut Client) -> Result<(), String> {
    let seq = client.global_seq();
    let test_instance_count = client.run_parameters().test_instance_count as usize;
    let mut bootstrap_addr = client
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

    // Wait until bootstrapping is done.
    client.barrier("bootstrap-ready", 1).await.unwrap();

    // Each peer should wait before attempting to bootstrap.
    // If all peers attempt to bootstrap at the same time, DHT won't
    // be updated in time for peers to discover each other.
    // We have no way to trigger bootstrapping later.
    tokio::time::sleep(tokio::time::Duration::from_secs(seq*4)).await;
    client.record_message(format!("[Node]: Bootstrapping to address {}", bootstrap_addr));

    // Init service.
    let local_key = libp2p::identity::Keypair::generate_ed25519();
    let service = node_init(local_key.clone(), config);
    let cmd_sender = service.command_sender();

    // Start service.
    tokio::task::spawn(async move { service.start().await.unwrap() });

    let peer_count = test_instance_count - 1;

    // Send a command to get the service's peers.
    let mut peers = HashSet::new();
    while peers.len() < peer_count {
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
        // client.record_success().await.expect("Success");
    }

    Ok(())
}