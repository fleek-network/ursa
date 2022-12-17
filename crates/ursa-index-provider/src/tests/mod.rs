use anyhow::Result;
use std::sync::Arc;

use crate::{config::ProviderConfig, engine::ProviderEngine};
use db::MemoryDB;
use libp2p::{identity::Keypair, PeerId};
use simple_logger::SimpleLogger;
use tracing::{info, log::LevelFilter};
use ursa_network::{NetworkConfig, UrsaService};
use ursa_store::UrsaStore;

pub fn setup_logger(level: LevelFilter) {
    if let Err(_) = SimpleLogger::new()
        .with_level(level)
        .with_utc_timestamps()
        .init()
    {
        info!("Logger already set. Ignore.")
    }
}

pub fn get_store() -> Arc<UrsaStore<MemoryDB>> {
    let db = Arc::new(MemoryDB::default());
    Arc::new(UrsaStore::new(Arc::clone(&db)))
}

pub fn provider_engine_init(port: u16) -> Result<(ProviderEngine<MemoryDB>, UrsaService<MemoryDB>, PeerId)> {
    setup_logger(LevelFilter::Info);
    let mut config = ProviderConfig::default();
    config.port = port;

    let mut network_config = NetworkConfig::default();
    network_config.swarm_addrs = vec!["/ip4/0.0.0.0/tcp/0".parse().unwrap()];;
    let keypair = Keypair::generate_ed25519();
    let peer_id = PeerId::from(keypair.public());

    let store = get_store();
    let index_store = get_store();

    let service = UrsaService::new(keypair.clone(), &network_config, Arc::clone(&store))?;

    let provider_engine = ProviderEngine::new(
        keypair.clone(),
        store,
        index_store,
        config.clone(),
        service.command_sender(),
    );
    Ok((provider_engine, service, peer_id))
}
