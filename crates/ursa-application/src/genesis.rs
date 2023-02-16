use anyhow::{Context, Result};
use resolve_path::PathResolveExt;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, path::PathBuf};

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
}
#[derive(Default, Serialize, Deserialize, Debug)]
pub struct GenesisContract {
    pub address: String,
    pub bytecode: String,
}

impl Genesis {
    /// Load the genesis file
    pub fn load() -> Result<Genesis> {
        let raw = include_str!("../genesis.toml");
        toml::from_str(&raw).context("Failed to parse genesis file")
    }
}
