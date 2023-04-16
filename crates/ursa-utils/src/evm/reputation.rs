use crate::transactions::AbciQueryQuery;
use anyhow::Result;
use ethers::abi::{AbiDecode, AbiEncode};
use ethers::contract::{EthAbiCodec, EthAbiType, EthCall, EthDisplay};
use ethers::types::{Address, Bytes, TransactionRequest, H160, U256};
use hex_literal::hex;
use tendermint_proto::abci::ResponseQuery;
use tokio::sync::{mpsc, oneshot};

use super::{query_application, send_txn_to_application};

pub const REPUTATION_ADDRESS: Address = H160(hex!("DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD"));

#[derive(Clone, Debug, Default, Eq, PartialEq, EthAbiType, EthAbiCodec)]
pub struct EpochScores {
    pub peer_id: String,
    pub measurements: ::std::vec::Vec<Measurement>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, EthAbiType, EthAbiCodec)]
pub struct Measurement {
    pub peer_id: String,
    pub bandwidth: u64,
    pub latency: u32,
    pub uptime: u128,
}

#[derive(Clone, Debug, Eq, PartialEq, EthCall, EthDisplay, Default)]
#[ethcall(
    name = "submitScores",
    abi = "submitScores(uint256,(string,(string,uint64,uint32,uint128)[]))"
)]
pub struct SubmitScoresCall {
    pub epoch: U256,
    pub scores: EpochScores,
}

#[derive(Clone, Debug, Eq, PartialEq, EthCall, EthDisplay, Default)]
#[ethcall(name = "getScores", abi = "getScores(uint256)")]
pub struct GetScoresCall {
    pub epoch: U256,
}

#[derive(Clone, Debug, Eq, PartialEq, EthAbiType, EthAbiCodec, Default)]
pub struct GetScoresReturn(pub Vec<EpochScores>);

pub async fn submit_scores_txn(
    scores: EpochScores,
    epoch: u64,
    mempool_address: String,
) -> Result<()> {
    let to = REPUTATION_ADDRESS;
    let data = SubmitScoresCall {
        epoch: epoch.into(),
        scores,
    }
    .encode();

    let txn = TransactionRequest::new().to(to).data(data);

    send_txn_to_application(mempool_address, txn).await
}

pub async fn get_scores(
    epoch: u64,
    tx_abci_queries: &mpsc::Sender<(oneshot::Sender<ResponseQuery>, AbciQueryQuery)>,
) -> Result<Vec<EpochScores>> {
    let to = REPUTATION_ADDRESS;
    let data = GetScoresCall {
        epoch: epoch.into(),
    }
    .encode();

    let response =
        query_application(tx_abci_queries, TransactionRequest::new().to(to).data(data)).await?;

    let scores = GetScoresReturn::decode(Bytes::from(response))?;

    Ok(scores.0)
}
