use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use ethers::abi::{AbiDecode, AbiParser, Token};
use ethers::contract::{EthAbiCodec, EthAbiType, EthCall, EthDisplay};
use ethers::core::types::U256;
use ethers::types::{Address, Bytes, TransactionRequest};
use fastcrypto::traits::EncodeDecodeBase64;
use lazy_static::lazy_static;
use narwhal_config::{Authority, Committee, WorkerCache, WorkerIndex, WorkerInfo};
use narwhal_crypto::{NetworkPublicKey, PublicKey};

lazy_static! {
    pub static ref EPOCH_ADDRESS: Address = "0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAC"
        .parse::<Address>()
        .unwrap();
    pub static ref EPOCH_INFO_CALL: Vec<u8> = vec![186, 188, 57, 79];
}

const SIGNAL_EPOCH_ABI: &str = "signalEpochChange(string):(bool)";

#[derive(Clone, Debug, Default, Eq, PartialEq, EthAbiType, EthAbiCodec)]
pub struct CommitteeMember {
    pub public_key: String,
    pub primary_address: String,
    pub network_key: String,
    pub workers: Vec<Worker>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, EthAbiType, EthAbiCodec)]
pub struct Worker {
    pub worker_address: String,
    pub worker_public_key: String,
    pub worker_mempool: String,
}

#[derive(Clone, Debug, Eq, PartialEq, EthAbiType, EthAbiCodec, Default)]
pub struct EpochInfoReturn {
    pub epoch: U256,
    pub current_epoch_end_ms: U256,
    pub committee_members: Vec<CommitteeMember>,
}

pub struct EpochInformation {
    authorities: BTreeMap<PublicKey, CommitteeMember>,
    epoch: u64,
}

#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    EthAbiType,
    EthAbiCodec,
    Default,
)]
pub struct SignalEpochChangeReturn(pub bool);

#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    EthCall,
    EthDisplay,
    Default,
)]
#[ethcall(
    name = "initialize",
    abi = "initialize(address,uint256,uint256,uint256)"
)]
pub struct InitializeCall {
    pub node_registry: Address,
    pub first_epoch_start: U256,
    pub epoch_duration: U256,
    pub max_committee_size: U256,
}

pub fn get_epoch_info_call() -> TransactionRequest {
    TransactionRequest::new()
        .to(*EPOCH_ADDRESS)
        .data(EPOCH_INFO_CALL.clone())
}

pub fn get_signal_epoch_change_call(public_key: String) -> TransactionRequest {
    // Safe unwrap, const valid ABI
    let function = AbiParser::default()
        .parse_function(SIGNAL_EPOCH_ABI)
        .unwrap();

    // Safe unwrap since only a String can be passed in here.
    let data = function.encode_input(&[Token::String(public_key)]).unwrap();

    TransactionRequest::new().to(*EPOCH_ADDRESS).data(data)
}

pub fn decode_epoch_info_return(output: Vec<u8>) -> Result<EpochInfoReturn> {
    EpochInfoReturn::decode(Bytes::from(output))
        .with_context(|| "Unable to decode the call results")
}

pub fn decode_committee(
    committee_members: Vec<CommitteeMember>,
    epoch: u64,
) -> (Committee, WorkerCache) {
    let epoch_info = EpochInformation {
        authorities: committee_members
            .iter()
            .filter_map(|authority| {
                if let Ok(public_key) = PublicKey::decode_base64(&authority.public_key) {
                    Some((public_key, authority.clone()))
                } else {
                    None
                }
            })
            .collect(),
        epoch,
    };

    (Committee::from(&epoch_info), WorkerCache::from(&epoch_info))
}

impl From<&EpochInformation> for Committee {
    fn from(output: &EpochInformation) -> Self {
        Committee {
            epoch: output.epoch,
            authorities: output
                .authorities
                .iter()
                .filter_map(|(public_key, authority)| {
                    if let Ok(authority) = Authority::try_from(authority) {
                        Some((public_key.clone(), authority))
                    } else {
                        None
                    }
                })
                .collect(),
        }
    }
}

impl TryFrom<&CommitteeMember> for Authority {
    type Error = anyhow::Error;
    fn try_from(member: &CommitteeMember) -> Result<Self> {
        let network_key = NetworkPublicKey::decode_base64(&member.network_key)
            .map_err(|_| anyhow!("Failed parsing network Key"))?;
        Ok(Authority {
            stake: 1,
            primary_address: member
                .primary_address
                .parse()
                .map_err(|_| anyhow!("Failed parsing primary address"))?,
            network_key,
        })
    }
}

impl From<&EpochInformation> for WorkerCache {
    fn from(output: &EpochInformation) -> Self {
        let worker_cache = WorkerCache {
            epoch: output.epoch,
            workers: output
                .authorities
                .iter()
                .map(|(key, authority)| {
                    let mut worker_index = BTreeMap::new();
                    authority
                        .workers
                        .iter()
                        .filter_map(|worker| {
                            Some(WorkerInfo {
                                name: NetworkPublicKey::decode_base64(&worker.worker_public_key)
                                    .ok()?,
                                transactions: worker.worker_mempool.parse().ok()?,
                                worker_address: worker.worker_address.parse().ok()?,
                            })
                        })
                        .enumerate()
                        .for_each(|(index, worker)| {
                            //Todo(dalton): Safe unwrap? The idea of the index overflowing u32 seems wild
                            worker_index.insert(index.try_into().unwrap(), worker);
                        });
                    (key.clone(), WorkerIndex(worker_index))
                })
                .collect(),
        };
        worker_cache
    }
}
