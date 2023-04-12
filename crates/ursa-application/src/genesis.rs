use anyhow::{Context, Result};
use ethers::types::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct Genesis {
    pub precompiles: Vec<GenesisContract>,
}
#[derive(Default, Serialize, Deserialize, Debug)]
pub struct GenesisContract {
    pub name: String,
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
