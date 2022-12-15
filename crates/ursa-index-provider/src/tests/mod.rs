mod engine_tests;
mod provider_tests;

use std::sync::Arc;
use anyhow:: Result;


use crate::{config::ProviderConfig, engine::ProviderEngine};
use db::MemoryDB;
use libp2p::{PeerId, identity::Keypair};
use simple_logger::SimpleLogger;
use tracing::{log::LevelFilter, info};
use ursa_network::{NetworkConfig, UrsaService};
use ursa_store::Store;


fn setup_logger(level: LevelFilter) {
    if let Err(_) = SimpleLogger::new()
        .with_level(level)
        .with_utc_timestamps()
        .init()
    {
        info!("Logger already set. Ignore.")
    }
}

fn get_store() -> Arc<Store<MemoryDB>> {
    let db = Arc::new(MemoryDB::default());
    Arc::new(Store::new(Arc::clone(&db)))
}

fn provider_engine_init() -> Result<(ProviderEngine<MemoryDB>, PeerId)> {
    setup_logger(LevelFilter::Debug);
    let mut config = ProviderConfig::default();
    config.port = 0;

    let network_config = NetworkConfig::default();
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
    Ok((provider_engine, peer_id))
}