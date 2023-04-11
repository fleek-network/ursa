use anyhow::{Context, Result};
use ethers::types::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct Genesis {
    #[serde(default)]
    pub hello: GenesisContract,
    #[serde(default)]
    pub token: GenesisContract,
    #[serde(default)]
    pub staking: GenesisContract,
    #[serde(default)]
    pub registry: GenesisContract,
    #[serde(default)]
    pub epoch: GenesisContract,
    #[serde(default)]
    pub reputation_scores: GenesisContract,
    #[serde(default)]
    pub rewards: GenesisContract,
    #[serde(default)]
    pub rewards_aggregator: GenesisContract
}
#[derive(Default, Serialize, Deserialize, Debug)]
pub struct GenesisContract {
    pub address: String,
    pub bytecode: String,
    pub init_params: Option<Bytes>,
}

impl Genesis {
    /// Load the genesis file.
    pub fn load() -> Result<Genesis> {
        let raw = include_str!("../genesis.toml");
        toml::from_str(raw).context("Failed to parse genesis file")
    }
}
