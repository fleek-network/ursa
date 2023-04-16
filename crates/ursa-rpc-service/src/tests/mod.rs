mod api_test;
mod server_test;

use anyhow::Result;
use axum::{headers::HeaderMap, routing::get, Router};
use db::MemoryDB;
use libp2p::identity::Keypair;
use simple_logger::SimpleLogger;
use std::sync::Arc;
use tendermint_proto::abci::ResponseQuery;
use tokio::sync::{
    mpsc::{channel, Sender as BoundedSender},
    oneshot,
};
use tracing::{log::LevelFilter, warn};
use ursa_utils::transactions::AbciQueryQuery;
use ursa_index_provider::{config::ProviderConfig, engine::ProviderEngine};
use ursa_network::{NetworkConfig, UrsaService};
use ursa_store::UrsaStore;

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

pub fn get_store() -> Arc<UrsaStore<MemoryDB>> {
    let db = Arc::new(MemoryDB::default());
    Arc::new(UrsaStore::new(Arc::clone(&db)))
}

type InitResult = Result<(
    UrsaService<MemoryDB>,
    ProviderEngine<MemoryDB>,
    Arc<UrsaStore<MemoryDB>>,
    String,
    BoundedSender<(oneshot::Sender<ResponseQuery>, AbciQueryQuery)>,
)>;

pub fn init() -> InitResult {
    let store = get_store();
    let network_config = NetworkConfig {
        swarm_addrs: vec!["/ip4/0.0.0.0/tcp/0".parse().unwrap()],
        bootstrap_nodes: vec![],
        ..Default::default()
    };
    let keypair = Keypair::generate_ed25519();
    let (sender, receiver) = channel(4096);
    let service = UrsaService::new(keypair.clone(), &network_config, Arc::clone(&store), sender)?;

    let provider_engine = ProviderEngine::new(
        keypair,
        Arc::clone(&store),
        get_store(),
        ProviderConfig::default(),
        service.command_sender(),
        vec!["/ip4/127.0.0.1/tcp/4069".parse().unwrap()],
        receiver,
    );
    let mempool_address = "/ip4/0.0.0.0/tcp/8102/http".to_string();
    let (abci_send, _abci_recieve) = channel(1000);
    Ok((service, provider_engine, store, mempool_address, abci_send))
}

pub async fn dummy_ipfs() -> Result<()> {
    let file: Vec<u8> = std::fs::read("../../test_files/test.car")?;

    let router = Router::new().route(
        "/ipfs/:cid",
        get(|| async move {
            let mut headers = HeaderMap::new();
            headers.insert("Content-Type", "application/vnd.ipfs.car".parse().unwrap());
            (headers, file.clone())
        }),
    );

    axum::Server::bind(&"0.0.0.0:9682".parse().unwrap())
        .serve(router.into_make_service())
        .await
        .map_err(|e| e.into())
}
