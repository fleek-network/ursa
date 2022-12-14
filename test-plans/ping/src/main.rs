use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use libp2p::swarm::{keep_alive, SwarmEvent, NetworkBehaviour};
use libp2p::*;
use std::collections::HashSet;
use std::time::Duration;
use ::ping::{run_ping, PingSwarm};
use ursa_network::{NetworkConfig, Behaviour as UrsaBehaviour};
use libipld::DefaultParams;
use ursa_store::{BitswapStorage, Store};
use db::{rocks::RocksDb, rocks_config::RocksDbConfig};
use std::sync::Arc;

#[async_std::main]
async fn main() -> Result<()> {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    //
    let db = RocksDb::open("test_db", &RocksDbConfig::default())
        .expect("Opening RocksDB must succeed");

    let db = Arc::new(db);
    let store = Arc::new(Store::new(Arc::clone(&db)));
    //

    let config = NetworkConfig::default();
    // let index_provider = Provider::new(local_key.clone(), index_store, provider_config.clone());
    let bitswap_store = BitswapStorage(store.clone());
    // ---

    let swarm = OrphanRuleWorkaround(Swarm::with_async_std_executor(
        development_transport(local_key.clone()).await?,
        Behaviour {
            keep_alive: keep_alive::Behaviour,
            ping: UrsaBehaviour::new(&local_key, &config, bitswap_store, None),
        },
        local_peer_id,
    ));

    run_ping(swarm).await?;

    Ok(())
}

#[derive(NetworkBehaviour)]
#[behaviour(prelude = "libp2p::swarm::derive_prelude")]
struct Behaviour {
    keep_alive: keep_alive::Behaviour,
    ping: UrsaBehaviour<DefaultParams>,
}

struct OrphanRuleWorkaround(Swarm<Behaviour>);

#[async_trait]
impl PingSwarm for OrphanRuleWorkaround {
    async fn listen_on(&mut self, address: &str) -> Result<()> {
        let id = self.0.listen_on(address.parse()?)?;

        loop {
            if let Some(SwarmEvent::NewListenAddr { listener_id, .. }) = self.0.next().await {
                if listener_id == id {
                    break;
                }
            }
        }

        Ok(())
    }

    fn dial(&mut self, address: &str) -> Result<()> {
        self.0.dial(address.parse::<Multiaddr>()?)?;

        Ok(())
    }

    async fn await_connections(&mut self, number: usize) {
        let mut connected = HashSet::with_capacity(number);

        while connected.len() < number {
            if let Some(SwarmEvent::ConnectionEstablished { peer_id, .. }) = self.0.next().await {
                connected.insert(peer_id);
            }
        }
    }

    async fn await_pings(&mut self, number: usize) {
        let mut received_pings = HashSet::with_capacity(number);

        while received_pings.len() < number {
            if let Some(SwarmEvent::Behaviour(BehaviourEvent::Ping(ursa_network::BehaviourEvent::Ping{peer}))) = self.0.next().await
            {
                received_pings.insert(peer);
            }
        }
    }

    async fn loop_on_next(&mut self) {
        loop {
            self.0.next().await;
        }
    }

    fn local_peer_id(&self) -> String {
        self.0.local_peer_id().to_string()
    }
}