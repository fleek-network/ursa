use anyhow::Result;
use ethers::types::{TransactionRequest, Address};
use ethers::abi::AbiEncode;
use narwhal_types::TransactionProto;
use bytes::Bytes;

use crate::contract_bindings::reputation_score_bindings::{EpochScores, SubmitScoresCall, ReputationScoresCalls };

pub const REPUTATION_ADDRESS: &str = "0xDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD";
pub fn submit_scores_txn(scores: EpochScores, epoch: u64) -> Result<TransactionProto>{
    let to = REPUTATION_ADDRESS.parse::<Address>().unwrap();
    let data = ReputationScoresCalls::SubmitScores(SubmitScoresCall{epoch: epoch.into(), scores}).encode();
    let txn = serde_json::to_vec(&TransactionRequest::new().to(to).data(data))?;

    Ok(TransactionProto {
        transaction: Bytes::from(txn),
    })

    
}