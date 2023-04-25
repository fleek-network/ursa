use std::sync::Arc;

use async_trait::async_trait;
use ethers::abi::AbiDecode;
use ethers::prelude::NameOrAddress;
use ethers::types::{Address, TransactionRequest};
use parking_lot::Mutex;
use revm::primitives::Output;
use revm::primitives::{AccountInfo, Bytecode};
use revm::{
    self,
    db::{CacheDB, EmptyDB},
    primitives::ExecutionResult,
};
use tm_abci::{
    ConsensusXX, Mempool, Query, RequestFinalizedBlock, RequestInfo, RequestInitChain,
    RequestQuery, ResponseDeliverTx, ResponseEndBlock, ResponseFinalizedBlock, ResponseInfo,
    ResponseInitChain, ResponseQuery, Snapshot,
};
use tm_protos::abci::{ConsensusParams, ResponseCommit};
use ursa_utils::evm::epoch_manager::{SignalEpochChangeReturn, EPOCH_ADDRESS};

use crate::genesis::Genesis;
use crate::types::{ApplicationError, Consensus, Info, Query as QueryType, QueryResponse, State};
use crate::{AbciDb, ExecutionResponse};

#[derive(Clone)]
pub struct App<Db> {
    pub consensus: Consensus<Db>,
    pub info: Info<Db>,
}

impl Default for App<CacheDB<EmptyDB>> {
    fn default() -> Self {
        Self::new()
    }
}

impl App<CacheDB<EmptyDB>> {
    pub fn new() -> Self {
        let state = State {
            db: CacheDB::new(EmptyDB()),
            block_height: Default::default(),
            app_hash: Default::default(),
            env: Default::default(),
        };

        let committed_state = Arc::new(Mutex::new(state.clone()));
        let current_state = Arc::new(Mutex::new(state));

        let consensus = Consensus {
            committed_state: committed_state.clone(),
            current_state,
        };

        let info = Info {
            state: committed_state,
        };

        App { consensus, info }
    }
}

impl<Db: AbciDb> App<Db> {
    async fn deliver_txn(&self, tx: Vec<u8>) -> ResponseDeliverTx {
        tracing::trace!("delivering tx");
        let mut state = self.consensus.current_state.lock();

        let mut tx: TransactionRequest = match serde_json::from_slice(&tx) {
            Ok(tx) => tx,
            Err(_) => {
                tracing::error!("could not decode request");
                return ResponseDeliverTx {
                    data: serde_json::to_vec(&ApplicationError::UnableToDecodeRequest).unwrap(),
                    ..Default::default()
                };
            }
        };

        let mut to_epoch_contract: bool = false;
        // Resolve the `to`.
        match tx.to {
            Some(NameOrAddress::Address(addr)) => {
                if addr == *EPOCH_ADDRESS {
                    to_epoch_contract = true;
                }
                tx.to = Some(addr.into())
            }
            None => (),
            _ => {
                return ResponseDeliverTx {
                    data: serde_json::to_vec(&ApplicationError::InvalidAddress).unwrap(),
                    ..Default::default()
                }
            }
        };

        let result = match state.execute(tx, false) {
            Ok(res) => res,
            Err(_) => {
                return ResponseDeliverTx {
                    data: serde_json::to_vec(&ApplicationError::ExecutionError).unwrap(),
                    ..Default::default()
                }
            }
        };
        tracing::trace!("executed tx");

        let mut response = ResponseDeliverTx {
            data: serde_json::to_vec(&ExecutionResponse::Transaction).unwrap(),
            ..Default::default()
        };

        if to_epoch_contract {
            if let ExecutionResult::Success {
                output: Output::Call(bytes),
                ..
            } = &result
            {
                let results = SignalEpochChangeReturn::decode(bytes)
                    .unwrap_or(SignalEpochChangeReturn(false));

                if results.0 {
                    // Tx response code 1 tells consensus to change epoch based on the results of this txn
                    response.code = 1;
                }
            }
        }
        response
    }
}

#[async_trait]
impl<Db: AbciDb> ConsensusXX for App<Db> {
    async fn finalized_block(&self, req: RequestFinalizedBlock) -> ResponseFinalizedBlock {
        let mut receipts = Vec::new();

        let mut epoch_changed = false;

        for tx in req.transactions {
            let response = self.deliver_txn(tx).await;

            if response.code == 1 {
                epoch_changed = true;
            }
            receipts.push(response.clone());
        }
        ResponseFinalizedBlock {
            tx_receipt: receipts.clone(),
            end_recepit: ResponseEndBlock {
                consensus_param_updates: match epoch_changed {
                    true => Some(ConsensusParams::default()),
                    false => None,
                },
                ..Default::default()
            },
        }
    }

    async fn commit(&self) -> ResponseCommit {
        tracing::trace!("taking lock");
        let current_state = self.consensus.current_state.lock().clone();
        let mut committed_state = self.consensus.committed_state.lock();
        *committed_state = current_state;
        tracing::trace!("committed");

        ResponseCommit {
            data: vec![], // (*committed_state).app_hash.clone(),
            retain_height: 0,
        }
    }

    async fn init_chain(&self, _init_chain_request: RequestInitChain) -> ResponseInitChain {
        tracing::trace!("initing the chain");
        let mut state = self.consensus.current_state.lock();

        // Load the bytecode for the contracts we need on genesis block.
        let genesis = Genesis::load().unwrap();

        genesis.precompiles.iter().for_each(|contract| {
            let address: Address = contract.address.parse().unwrap_or_else(|_| {
                panic!(
                    "Invalid genesis.toml: Invalid address({}) on precompile {}",
                    contract.address, contract.name
                )
            });
            let bytes = hex::decode(&contract.bytecode).unwrap_or_else(|_| {
                panic!(
                    "Invalid genesis.toml: Invalid bytecode on precompile {}",
                    contract.name
                )
            });

            //Insert into db
            state.db.insert_account_info(
                address.to_fixed_bytes().into(),
                AccountInfo {
                    code: Some(Bytecode::new_raw(bytes.into())),
                    ..Default::default()
                },
            );

            // If precompile has init_params call init
            if let Some(bytes) = &contract.init_params {
                // Call the init transactions.
                let tx = TransactionRequest {
                    to: Some(address.into()),
                    data: Some(bytes.clone()),
                    ..Default::default()
                };

                let _results = state.execute(tx, false).unwrap_or_else(|_| {
                    panic!(
                        "Invalid genesis.toml: Invalid init params on precompile {}",
                        contract.name
                    )
                });
            }
        });

        *self.consensus.committed_state.lock() = state.clone();
        drop(state);

        ResponseInitChain::default()
    }
}

#[async_trait]
impl<Db: AbciDb> Query for App<Db> {
    async fn info(&self, _request: RequestInfo) -> ResponseInfo {
        let state = self.info.state.lock();

        ResponseInfo {
            data: Default::default(),
            version: Default::default(),
            app_version: Default::default(),
            last_block_height: state.block_height,
            last_block_app_hash: state.app_hash.clone(),
        }
    }

    // Replicate the eth_call interface.
    async fn query(&self, query_request: RequestQuery) -> ResponseQuery {
        let mut state = self.info.state.lock();

        let query: QueryType = match serde_json::from_slice(&query_request.data) {
            Ok(tx) => tx,
            // no-op just logger
            Err(_) => {
                return ResponseQuery {
                    value: serde_json::to_vec(&ApplicationError::UnableToDecodeRequest).unwrap(),
                    ..Default::default()
                };
            }
        };

        let resp = match query {
            QueryType::EthCall(tx) => {
                let result = match state.execute(tx, true) {
                    Ok(res) => res,
                    Err(_) => {
                        return ResponseQuery {
                            value: serde_json::to_vec(&ApplicationError::ExecutionError).unwrap(),
                            ..Default::default()
                        }
                    }
                };

                if let ExecutionResult::Success {
                    output: Output::Call(bytes),
                    ..
                } = result
                {
                    bytes.to_vec()
                } else {
                    "Call was not succesful".into()
                }
            }
            QueryType::Balance(address) => match state.db.basic(address.to_fixed_bytes().into()) {
                Ok(info) => {
                    let res = QueryResponse::Balance(info.unwrap_or_default().balance);
                    serde_json::to_vec(&res).unwrap()
                }
                _ => "error retrieving balance".into(),
            },
        };

        ResponseQuery {
            key: query_request.data,
            value: resp,
            ..Default::default()
        }
    }
}

impl<Db: AbciDb> Mempool for App<Db> {}

impl<Db: AbciDb> Snapshot for App<Db> {}
