use crate::evm::epoch_manager::Worker;

use ethers::contract::{EthAbiCodec, EthAbiType, EthCall, EthDisplay};
use ethers::types::Address;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref REGISTRY_ADDRESS: Address = "0xCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC"
        .parse::<Address>()
        .unwrap();
}

#[derive(Clone, Debug, Default, Eq, PartialEq, EthAbiType, EthAbiCodec)]
pub struct NodeInfo {
    pub owner: ethers::core::types::Address,
    pub primary_public_key: String,
    pub primary_address: String,
    pub network_key: String,
    pub workers: ::std::vec::Vec<Worker>,
}

#[derive(Clone, Debug, Eq, PartialEq, EthCall, EthDisplay, Default)]
#[ethcall(
    name = "initialize",
    abi = "initialize((address,string,string,string,(string,string,string)[])[])"
)]
pub struct InitializeCall {
    pub genesis_committee: Vec<NodeInfo>,
}
