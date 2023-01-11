use std::sync::Arc;

use db::MemoryDB;
use libp2p::identity::Keypair;
use libp2p::Multiaddr;
use simple_logger::SimpleLogger;
use tracing::{log::LevelFilter, warn};

use ursa_index_provider::{config::ProviderConfig, engine::ProviderEngine};
use ursa_network::{NetworkConfig, UrsaService};
use ursa_store::UrsaStore;

mod api_test;
mod server_test;

pub fn setup_logger() {
    let level = LevelFilter::Debug;
    if let Err(err) = SimpleLogger::new()
        .with_level(level)
        .with_utc_timestamps()
        .init()
    {
        warn!("Logger already set {:?}:", err)
    }
}

pub fn get_store() -> UrsaStore<MemoryDB> {
    UrsaStore::new(Arc::new(MemoryDB::default()))
}

type InitResult = anyhow::Result<(
    UrsaService<MemoryDB>,
    ProviderEngine<MemoryDB>,
    UrsaStore<MemoryDB>,
)>;

pub fn init() -> InitResult {
    let store = get_store();
    let network_config = NetworkConfig {
        swarm_addrs: vec!["/ip4/0.0.0.0/tcp/0".parse().unwrap()],
        ..Default::default()
    };
    let keypair = Keypair::generate_ed25519();
    let service = UrsaService::new(keypair.clone(), &network_config, store.clone())?;
    let server_address = Multiaddr::try_from("/ip4/0.0.0.0/tcp/0").unwrap();

    let provider_engine = ProviderEngine::new(
        keypair,
        store.clone(),
        get_store(),
        ProviderConfig::default(),
        service.command_sender(),
        server_address,
    );

    Ok((service, provider_engine, store))
}
