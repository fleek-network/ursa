use ethers::abi::AbiEncode;
use ethers::prelude::Address;
use ethers::prelude::U256 as UInt256;
use ethers::types::Bytes;
use std::time::SystemTime;
use std::{env, fs};
use ursa_application::genesis::Genesis;
use ursa_utils::contract_bindings::epoch::{EpochCalls, InitializeCall};
use ursa_utils::contract_bindings::node_registry::{
    InitializeCall as RegistryInitCall, NodeInfo, NodeRegistryCalls,
};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let epoch_time = match args.get(1) {
        Some(time) => time,
        None => "300000",
    };
    let registry_address: Address = "0xCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC"
        .parse()
        .unwrap();

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let initialize_call = InitializeCall {
        node_registry: registry_address,
        first_epoch_start: UInt256::from_dec_str(&now.to_string()).unwrap(),
        epoch_duration: UInt256::from_dec_str(epoch_time).unwrap(),
        max_committee_size: UInt256::from_dec_str("100").unwrap(),
    };

    let epoch_bytes = EpochCalls::Initialize(initialize_call).encode();

    let mut array = Vec::new();
    array.push(NodeInfo {
        owner: registry_address.clone(),
        primary_public_key:"l0Jel6KEFG7H6sV2nWKOQxDaMKWMeiUBqK5VHKcStWrLPHAANRB+dt7gp0jQ7ooxEaI7ukOQZk6U5vcL7ESHA1J/iAWQ7YNO/ZCvR1pfWfcTNBONIzeiUWAN+iyKfV10".to_string(),
        primary_address:"/ip4/127.0.0.1/udp/38000".to_string(),
        network_key:"EfP5ha4KNRu/qkfIuF3lWK7GPeP5IqPKP8esnM0mo2s=".to_string(),
        worker_address:"/ip4/127.0.0.1/udp/38101/http".to_string(),
        worker_public_key:"Z/wR6iweRxwJwhJxLjyyTKzFGXZZdYBJXdKzukjitwM=".to_string(),
        worker_mempool: "/ip4/127.0.0.1/tcp/38102/http".to_string()
    });
    array.push(NodeInfo {
        owner: registry_address.clone(),
        primary_public_key:"qipezx5pzmPFWICevMx+SL5+bIjG4yw3A9ieYKKwf2wTEvK0gMRYOln9+KmbNRB3FRbVQBLuCEWIHT0V9GxATT9VeJ+HT88vh/B/6dj7CbWBdWbZ4QXzo0q+uyGchopl".to_string(),
        primary_address:"/ip4/127.0.0.1/udp/28000".to_string(),
        network_key:"Ke9zCMItF3ryUI8ZCLj5oy97zFuT1eaaigZ1YuTmeuI=".to_string(),
        worker_address:"/ip4/127.0.0.1/udp/28101/http".to_string(),
        worker_public_key:"Lgcg7TEXuXLhZXrtsQLZPh9qCDelWbTaa/KYY79tll8=".to_string(),
        worker_mempool:"/ip4/127.0.0.1/tcp/28102/http".to_string()
    });

    array.push(NodeInfo {
        owner: registry_address.clone(),
        primary_public_key:"mAT9YIKvBhmi8L3e50/pr/dna1EaYELYTBuTwWgc7qoj5GrJ86/+7aX2mubVYOMxCUr2CO4H5nCgBBssNJ2u/buAPeM4MmEl+wmPgrX0uAAMzHyUOYQKVHCBFeFWsHJR".to_string(),
        primary_address:"/ip4/127.0.0.1/udp/18000".to_string(),
        network_key:"OEPeT9W4mz4WG2tGzqmtDavt2NGIfcmDsb1Qbzg5hoY=".to_string(),
        worker_address:"/ip4/127.0.0.1/udp/18101/http".to_string(),
        worker_public_key:"xD114IYSBkc/5R2uXU2h9IZw6WqY1X0rynnt60eG7Q0=".to_string(),
        worker_mempool: "/ip4/127.0.0.1/tcp/18102/http".to_string()
    });

    array.push(NodeInfo {
        owner: registry_address.clone(),
        primary_public_key:"tgLk6pAP2M9GrlrZnB9Y2zOCSbtAhc+bVXtnkX2oBZa0iH5JNPhyQWO0YhAHCisjCMbGSQEm8D2q/4THiCQOmXOXETHQ5Mb1SJQ5PvIRaavZ/UZm5KVYOfj2Zg415aUa".to_string(),
        primary_address:"/ip4/127.0.0.1/udp/8000".to_string(),
        network_key:"s6e/MVpeOIMxLxnTdYHFKefX+XGlA7ZSdFrdetfnjtI=".to_string(),
        worker_address:"/ip4/127.0.0.1/udp/8101/http".to_string(),
        worker_public_key:"FzAP+CZcG2HTzYHaGzGsamJDpw1nq0CRnmzl5SgUqmE=".to_string(),
        worker_mempool:"/ip4/127.0.0.1/tcp/8102/http".to_string()
    });

    let init_call = RegistryInitCall {
        genesis_committee: array,
    };

    let registry_bytes = NodeRegistryCalls::Initialize(init_call).encode();

    let mut genesis = Genesis::load().unwrap();
    genesis.epoch.init_params = Some(Bytes::from(epoch_bytes));
    genesis.registry.init_params = Some(Bytes::from(registry_bytes));

    let genesis_toml = toml::to_string(&genesis).unwrap();
    fs::write(
        env::current_dir()
            .unwrap()
            .join("crates/ursa-application/genesis.toml"),
        genesis_toml,
    )
    .unwrap();
}
