use anyhow::{Context, Result};
use fastcrypto::traits::EncodeDecodeBase64;
use multiaddr::Multiaddr;
use serde::{Deserialize, Serialize};

use crate::interface::application::{BLSPublicKey, NodeInfo, PublicKey, Worker};

#[derive(Serialize, Deserialize)]
pub struct Genesis {
    pub epoch_start: u64,
    pub epoch_time: u64,
    pub committee_size: u64,
    pub min_stake: u64,
    pub eligibility_time: u64,
    pub lock_time: u64,
    pub protocol_percentage: u64,
    pub max_inflation: u64,
    pub min_inflation: u64,
    pub consumer_rebate: u64,
    pub committee: Vec<GenesisCommittee>,
    pub service: Vec<GenesisService>,
    pub account: Vec<GenesisAccount>,
}

#[derive(Serialize, Deserialize)]
pub struct GenesisAccount {
    pub public_key: String,
    pub flk_balance: u64,
    pub bandwidth_balance: u64,
    pub staked: u64,
}

#[derive(Serialize, Deserialize)]
pub struct GenesisService {
    pub id: u64,
    pub commodity_price: u64,
}

#[derive(Serialize, Deserialize)]
pub struct GenesisCommittee {
    owner: String,
    primary_public_key: String,
    primary_address: String,
    network_key: String,
    worker_address: String,
    worker_public_key: String,
    worker_mempool: String,
    pub staking: Option<u64>,
}

impl Genesis {
    /// Load the genesis file.
    pub fn load() -> Result<Genesis> {
        let raw = include_str!("../genesis.toml");
        toml::from_str(raw).context("Failed to parse genesis file")
    }
}

impl From<&GenesisCommittee> for NodeInfo {
    fn from(value: &GenesisCommittee) -> Self {
        let owner = PublicKey::decode_base64(&value.owner).unwrap();
        let public_key = BLSPublicKey::decode_base64(&value.primary_public_key).unwrap();
        let network_key = PublicKey::decode_base64(&value.network_key).unwrap();
        let domain: Multiaddr = value.primary_address.parse().unwrap();

        let worker = Worker {
            public_key: PublicKey::decode_base64(&value.worker_public_key).unwrap(),
            address: value.worker_address.parse().unwrap(),
            mempool: value.worker_mempool.parse().unwrap(),
        };

        NodeInfo {
            owner,
            public_key,
            network_key,
            domain,
            workers: [worker].to_vec(),
        }
    }
}
