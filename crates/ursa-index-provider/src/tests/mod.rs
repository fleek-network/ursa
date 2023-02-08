use anyhow::Result;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use crate::{config::ProviderConfig, engine::ProviderEngine};
use db::MemoryDB;
use libp2p::{identity::Keypair, Multiaddr, PeerId};
use simple_logger::SimpleLogger;
use tokio::task;
use tracing::{info, log::LevelFilter};
use ursa_network::{NetworkConfig, UrsaService};
use ursa_store::UrsaStore;

pub fn setup_logger(level: LevelFilter) {
    if SimpleLogger::new()
        .with_level(level)
        .with_utc_timestamps()
        .init()
        .is_err()
    {
        info!("Logger already set. Ignore.")
    }
}

pub fn get_store() -> Arc<UrsaStore<MemoryDB>> {
    let db = Arc::new(MemoryDB::default());
    Arc::new(UrsaStore::new(Arc::clone(&db)))
}

pub fn provider_engine_init(
    port: u16,
) -> Result<(ProviderEngine<MemoryDB>, UrsaService<MemoryDB>, PeerId)> {
    setup_logger(LevelFilter::Info);

    let network_config = NetworkConfig {
        swarm_addrs: vec!["/ip4/0.0.0.0/tcp/0".parse().unwrap()],
        ..Default::default()
    };
    let keypair = Keypair::generate_ed25519();
    let peer_id = PeerId::from(keypair.public());

    let store = get_store();
    let index_store = get_store();

    let service = UrsaService::new(keypair.clone(), &network_config, Arc::clone(&store))?;

    let server_address = Multiaddr::try_from("/ip4/0.0.0.0/tcp/0").unwrap();

    let provider_engine = ProviderEngine::new(
        keypair,
        store,
        index_store,
        ProviderConfig::default(),
        service.command_sender(),
        server_address,
        "/ip4/127.0.0.1/tcp/4069".parse().unwrap(),
    );

    let router = provider_engine.router();
    task::spawn(async move {
        // startup standalone http server for index provider
        axum::Server::bind(&SocketAddr::from_str(&format!("0.0.0.0:{port}")).unwrap())
            .serve(router.into_make_service())
            .await
            .expect("Failed to start provider server");
    });

    Ok((provider_engine, service, peer_id))
}
