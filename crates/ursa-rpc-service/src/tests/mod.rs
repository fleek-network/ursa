mod api_test;
mod server_test;

use db::MemoryDB;
use libp2p::identity::Keypair;
use simple_logger::SimpleLogger;
use std::sync::Arc;
use tracing::{error, log::LevelFilter};
use ursa_index_provider::{config::ProviderConfig, engine::ProviderEngine};
use ursa_network::{NetworkConfig, UrsaService};
use ursa_store::UrsaStore;

pub fn setup_logger(level: LevelFilter) {
    if let Err(err) = SimpleLogger::new()
        .with_level(level)
        .with_utc_timestamps()
        .init()
    {
        error!("Logger already set {:?}:", err)
    }
}

pub fn get_store() -> Arc<UrsaStore<MemoryDB>> {
    let db = Arc::new(MemoryDB::default());
    Arc::new(UrsaStore::new(Arc::clone(&db)))
}

type InitResult = anyhow::Result<(
    UrsaService<MemoryDB>,
    ProviderEngine<MemoryDB>,
    Arc<UrsaStore<MemoryDB>>,
)>;

pub fn init() -> InitResult {
    let store = get_store();
    let network_config = NetworkConfig::default();
    let keypair = Keypair::generate_ed25519();
    let service = UrsaService::new(keypair.clone(), &network_config, Arc::clone(&store))?;

    let provider_engine = ProviderEngine::new(
        keypair,
        Arc::clone(&store),
        get_store(),
        ProviderConfig::default(),
        service.command_sender(),
    );

    Ok((service, provider_engine, store))
}
