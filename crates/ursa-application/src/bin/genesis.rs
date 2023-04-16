use ethers::abi::AbiEncode;
use ethers::prelude::Address;
use ethers::prelude::U256 as UInt256;
use ethers::types::Bytes;
use serde::Deserialize;
use serde::Serialize;
use std::time::SystemTime;
use std::{env, fs};
use ursa_application::genesis::Genesis;
use ursa_utils::evm::epoch_manager::{InitializeCall, Worker};
use ursa_utils::evm::node_registry::{
    InitializeCall as RegistryInitCall, NodeInfo, REGISTRY_ADDRESS,
};

#[derive(Serialize, Deserialize, Debug)]
struct GenesisNode {
    owner: Address,
    primary_public_key: String,
    primary_address: String,
    network_key: String,
    worker_address: String,
    worker_public_key: String,
    worker_mempool: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GenesisCommittee {
    committee: Vec<GenesisNode>,
}

const GENESIS_PATH: &str = "crates/ursa-application/genesis.toml";

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let epoch_time = match args.get(1) {
        Some(time) => time,
        None => "300000",
    };
    let registry_address: Address = *REGISTRY_ADDRESS;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let epoch_bytes = InitializeCall {
        node_registry: registry_address,
        first_epoch_start: UInt256::from_dec_str(&now.to_string()).unwrap(),
        epoch_duration: UInt256::from_dec_str(epoch_time).unwrap(),
        max_committee_size: UInt256::from_dec_str("100").unwrap(),
    }
    .encode();

    let raw = include_str!("./genesis_committee.toml");
    let genesis: GenesisCommittee = toml::from_str(raw).unwrap();

    let genesis_committee: Vec<NodeInfo> = genesis
        .committee
        .iter()
        .map(|node| NodeInfo {
            owner: node.owner,
            primary_public_key: node.primary_public_key.to_string(),
            primary_address: node.primary_address.to_string(),
            network_key: node.network_key.to_string(),
            workers: [Worker {
                worker_address: node.worker_address.to_string(),
                worker_mempool: node.worker_mempool.to_string(),
                worker_public_key: node.worker_public_key.to_string(),
            }]
            .to_vec(),
        })
        .collect();

    let registry_bytes = RegistryInitCall { genesis_committee }.encode();

    let mut genesis = Genesis::load().unwrap();

    genesis.precompiles.iter_mut().for_each(|contract| {
        if contract.name == *"registry" {
            contract.init_params = Some(Bytes::from(registry_bytes.clone()));
        } else if contract.name == *"epoch" {
            contract.init_params = Some(Bytes::from(epoch_bytes.clone()))
        }
    });

    let genesis_toml = toml::to_string(&genesis).unwrap();
    fs::write(env::current_dir().unwrap().join(GENESIS_PATH), genesis_toml).unwrap();
}
