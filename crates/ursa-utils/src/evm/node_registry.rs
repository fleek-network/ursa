use crate::evm::epoch_manager::Worker;

use ethers::contract::{EthAbiCodec, EthAbiType, EthCall, EthDisplay};
use ethers::types::{Address, H160};
use hex_literal::hex;

pub const REGISTRY_ADDRESS: Address = H160(hex!("0000000000000000000000000000000000000096"));

#[derive(Clone, Debug, Default, Eq, PartialEq, EthAbiType, EthAbiCodec)]
pub struct NodeInfo {
    pub owner: Address,
    pub primary_public_key: String,
    pub primary_address: String,
    pub network_key: String,
    pub workers: Vec<Worker>,
}

#[derive(Clone, Debug, Eq, PartialEq, EthCall, EthDisplay, Default)]
#[ethcall(
    name = "initialize",
    abi = "initialize((address,string,string,string,(string,string,string)[])[])"
)]
pub struct InitializeCall {
    pub genesis_committee: Vec<NodeInfo>,
}
