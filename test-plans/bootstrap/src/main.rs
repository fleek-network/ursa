use ursa_network::{UrsaService, NetworkConfig};
use ursa_store::Store;
use ursa_index_provider::provider::Provider;
use std::sync::Arc;
use db::rocks_config::RocksDbConfig;
use db::rocks::RocksDb;
use libp2p::PeerId;
use ursa_index_provider::config::ProviderConfig;
#[tokio::main]
async fn main() {
    let client = testground::client::Client::new_and_init().await.unwrap();
    let mut config = NetworkConfig::default();

    // The first one that arrives will be the bootstrapper.
    let seq = client.global_seq();
    if seq == 1 {
        config.bootstrap_nodes = vec![];
        config.swarm_addr = "/ip4/0.0.0.0/tcp/6009".parse().unwrap();
    }

    // Wait until bootstrapping is done.
    client.barrier("bootstrap-done", client.run_parameters().test_instance_count).await.unwrap();

    // Start service.
    let local_key = libp2p::identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    let service = node_init(local_key, config);

    // TODO: Publish and verified that it was published...or something like this
    // service.command_send(PUBLISH_MESSAGE)
    // loop {
    //     select! {
    //             event = service.swarm.next() => {
    //                 let event = event.ok_or_else(|| anyhow!("Swarm Event invalid!"))?;
    //                 service.handle_swarm_event(event).expect("Handle swarm event.");
    //             },
    //             event_received = service.swarm.event_received() => {
    //                 If let Gossipsub(message) = event_received {
    //                      if message == PUBLISH_MESSAGE {
    //                          break;
    //                      }
    //                 }
    //             },
    //         }
    // }

    // client.success();

}


fn node_init(keypair: libp2p::identity::Keypair, config: NetworkConfig) -> UrsaService<RocksDb> {
    let db = RocksDb::open("test_db", &RocksDbConfig::default())
        .expect("Opening RocksDB must succeed");
    let provider_db = RocksDb::open("index_provider_db", &RocksDbConfig::default())
        .expect("Opening RocksDB must succeed");

    let db = Arc::new(db);
    let store = Arc::new(Store::new(Arc::clone(&db)));
    let index_store = Arc::new(Store::new(Arc::clone(&Arc::new(provider_db))));

    let provider_config = ProviderConfig::default();
    let index_provider = Provider::new(keypair.clone(), index_store, provider_config.clone());
    UrsaService::new(keypair, &config, Arc::clone(&store), index_provider)
}