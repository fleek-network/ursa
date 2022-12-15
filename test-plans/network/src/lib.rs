use anyhow::{anyhow, Result};
use db::MemoryDB;
use futures::future::ready;
use futures::{FutureExt, StreamExt};
use libipld::DefaultParams;
use libp2p::swarm::derive_prelude::FromSwarm;
use libp2p::swarm::SwarmEvent::ConnectionEstablished;
use libp2p::swarm::{SwarmBuilder, SwarmEvent};
use libp2p::{identity, tokio_development_transport, Multiaddr, PeerId, Swarm};
use libp2p_bitswap::BitswapStore;
use log::info;
use std::collections::HashSet;
use std::sync::Arc;
use testground::client::Client;
use tokio::select;
use ursa_network::behaviour::{Behaviour, BehaviourEvent};
use ursa_network::NetworkConfig;
use ursa_store::{BitswapStorage, Store};

pub const LISTENING_PORT: u16 = 1234;
pub const BOOTSTRAP_COUNT: u64 = 2;

type DefaultSwarm = Swarm<Behaviour<DefaultParams>>;
type DefaultEvent = BehaviourEvent<DefaultParams>;
pub struct TestSwarm {
    pub swarm: DefaultSwarm,
    pub client: Client,
    pub local_addr: Multiaddr,
}

impl TestSwarm {
    pub async fn new() -> Result<Self> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.clone().public());
        let bitswap_store = BitswapStorage(Arc::new(Store::new(Arc::new(MemoryDB::default()))));

        let client = Client::new_and_init()
            .await
            .map_err(|e| anyhow!("Failed to initialize testground client: {}", e.to_string()))?;

        let local_addr: Multiaddr = match if_addrs::get_if_addrs()
            .unwrap()
            .into_iter()
            .find(|iface| iface.name == "eth1")
            .unwrap()
            .addr
            .ip()
        {
            std::net::IpAddr::V4(addr) => format!("/ip4/{}/tcp/{}", addr, LISTENING_PORT),
            std::net::IpAddr::V6(_) => unimplemented!(),
        }
        .parse()?;

        let mut config = NetworkConfig {
            mdns: false,
            relay_server: false,
            autonat: false,
            relay_client: false,
            bootstrapper: false,
            swarm_addrs: vec![local_addr.clone()],
            bootstrap_nodes: vec![],
            database_path: Default::default(),
            identity: "".to_string(),
            keystore_path: Default::default(),
        };

        let mut swarm = SwarmBuilder::with_tokio_executor(
            tokio_development_transport(local_key.clone()).unwrap(),
            Behaviour::new(&local_key, &Default::default(), bitswap_store, None),
            local_peer_id,
        )
        .build();

        // Swarm listen and wait for established
        let id = swarm.listen_on(local_addr.clone())?;
        loop {
            if let Some(SwarmEvent::NewListenAddr { listener_id, .. }) = swarm.next().await {
                if listener_id == id {
                    break;
                }
            }
        }

        Ok(TestSwarm {
            swarm,
            client,
            local_addr,
        })
    }

    pub fn dial(&mut self, address: &str) -> Result<()> {
        self.swarm
            .dial(address.parse::<Multiaddr>()?)
            .map_err(|e| e.into())
    }

    pub async fn await_connections(&mut self, number: usize) {
        let mut connected = HashSet::with_capacity(number);

        while connected.len() < number {
            if let Some(SwarmEvent::ConnectionEstablished { peer_id, .. }) = self.swarm.next().await
            {
                connected.insert(peer_id);
            }
        }
    }

    pub async fn await_pings(&mut self, number: usize) {
        let mut received_pings = HashSet::with_capacity(number);

        while received_pings.len() < number {
            if let Some(SwarmEvent::Behaviour(BehaviourEvent::Ping(libp2p::ping::Event {
                peer,
                result,
            }))) = self.swarm.next().await
            {
                if result.is_ok() {
                    received_pings.insert(peer);
                }
            }
        }
    }

    pub async fn drive_until_signal<S: ToString>(&mut self, tag: S) -> Result<()> {
        let tag = tag.to_string();
        info!(
            "Signal and wait for all peers to signal being done with \"{}\".",
            tag
        );

        // we loop on swarm events until `signal_and_wait` finishes.
        select!(
            _ = ready(loop {
                    self.swarm.next().await;
                }) => {},
            _ = self.client
                .signal_and_wait(tag, self.client.run_parameters().test_instance_count)
                .boxed_local() => {}
        );

        Ok(())
    }

    pub fn local_peer_id(&self) -> PeerId {
        self.swarm.local_peer_id().clone()
    }
}
