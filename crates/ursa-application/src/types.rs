use crate::genesis::Genesis;
use crate::AbciDb;
use abci::{
    async_api::{
        Consensus as ConsensusTrait, Info as InfoTrait, Mempool as MempoolTrait,
        Snapshot as SnapshotTrait,
    },
    async_trait,
    types::*,
};
use anyhow::{bail, Result};
use ethers::abi::AbiDecode;
use ethers::prelude::NameOrAddress;
use ethers::types::{Address, TransactionRequest};
use revm::primitives::{AccountInfo, Bytecode, CreateScheme, TransactTo, B160, U256};
use revm::{
    self,
    db::{CacheDB, EmptyDB},
    primitives::{Env, ExecutionResult, TxEnv},
    Database, DatabaseCommit,
};
use revm::{db::DatabaseRef, primitives::Output};
use std::sync::Arc;
use tokio::sync::Mutex;
use ursa_utils::contract_bindings::epoch_bindings::SignalEpochChangeReturn;
use ursa_utils::transactions::EPOCH_ADDRESS;

#[derive(Clone, Debug)]
pub struct State<Db> {
    pub block_height: i64,
    pub app_hash: Vec<u8>,
    pub db: Db,
    pub env: Env,
}

pub trait WithGenesisDb {
    fn insert_account_info(&mut self, address: B160, info: AccountInfo);
}

impl<Db: DatabaseRef> WithGenesisDb for CacheDB<Db> {
    #[inline(always)]
    fn insert_account_info(&mut self, address: B160, info: AccountInfo) {
        CacheDB::<Db>::insert_account_info(self, address, info);
    }
}

impl Default for State<CacheDB<EmptyDB>> {
    fn default() -> Self {
        Self {
            block_height: 0,
            app_hash: Vec::new(),
            db: CacheDB::new(EmptyDB()),
            env: Default::default(),
        }
    }
}

impl<Db: DatabaseCommit + Database> State<Db> {
    async fn execute(
        &mut self,
        tx: TransactionRequest,
        read_only: bool,
    ) -> Result<ExecutionResult> {
        let mut evm = revm::EVM::new();
        evm.env = self.env.clone();
        evm.env.tx = TxEnv {
            caller: tx.from.unwrap_or_default().to_fixed_bytes().into(),
            transact_to: match tx.to {
                Some(NameOrAddress::Address(inner)) => {
                    TransactTo::Call(inner.to_fixed_bytes().into())
                }
                Some(NameOrAddress::Name(_)) => bail!("not allowed"),
                None => TransactTo::Create(CreateScheme::Create),
            },
            data: tx.data.clone().unwrap_or_default().0,
            chain_id: Some(self.env.cfg.chain_id.try_into().unwrap()),
            nonce: Some(tx.nonce.unwrap_or_default().as_u64()),
            value: tx.value.unwrap_or_default().into(),
            gas_price: tx.gas_price.unwrap_or_default().into(),
            gas_priority_fee: Some(tx.gas_price.unwrap_or_default().into()),
            gas_limit: u64::MAX,
            access_list: vec![],
        };
        evm.database(&mut self.db);

        let results = match evm.transact() {
            Ok(data) => data,
            Err(_) => bail!("theres an err"),
        };
        if !read_only {
            self.db.commit(results.state);
        };
        Ok(results.result)
    }
}

pub struct Consensus<Db> {
    pub committed_state: Arc<Mutex<State<Db>>>,
    pub current_state: Arc<Mutex<State<Db>>>,
}

impl<Db: Clone> Consensus<Db> {
    pub fn new(state: State<Db>) -> Self {
        let committed_state = Arc::new(Mutex::new(state.clone()));
        let current_state = Arc::new(Mutex::new(state));

        Consensus {
            committed_state,
            current_state,
        }
    }
}

#[async_trait]
impl<Db: AbciDb> ConsensusTrait for Consensus<Db> {
    #[tracing::instrument(skip(self))]
    async fn init_chain(&self, _init_chain_request: RequestInitChain) -> ResponseInitChain {
        tracing::trace!("initing the chain");
        let mut state = self.current_state.lock().await;

        // Load the bytecode for the contracts we need on genesis block.
        let genesis = Genesis::load().unwrap();

        let token_bytes = hex::decode(genesis.token.bytecode).unwrap();
        let staking_bytes = hex::decode(genesis.staking.bytecode).unwrap();
        let registry_bytes = hex::decode(genesis.registry.bytecode).unwrap();
        let epoch_bytes = hex::decode(genesis.epoch.bytecode).unwrap();
        let hello_bytes = hex::decode(genesis.hello.bytecode).unwrap();
        let rep_bytes = hex::decode(genesis.reputation_scores.bytecode).unwrap();
        let rewards_bytes = hex::decode(genesis.rewards.bytecode).unwrap();
        let rewards_agg_bytes = hex::decode(genesis.rewards_aggregator.bytecode).unwrap();
        // Parse addresses for contracts.
        let token_address: Address = genesis.token.address.parse().unwrap();
        let staking_address: Address = genesis.staking.address.parse().unwrap();
        let registry_address: Address = genesis.registry.address.parse().unwrap();
        let epoch_address: Address = genesis.epoch.address.parse().unwrap();
        let rep_address: Address = genesis.reputation_scores.address.parse().unwrap();
        let rewards_address: Address = genesis.rewards.address.parse().unwrap();
        let rewards_agg_address: Address = genesis.rewards_aggregator.address.parse().unwrap();

        // Build the account info for the contracts.
        let token_contract = AccountInfo {
            code: Some(Bytecode::new_raw(token_bytes.into())),
            ..Default::default()
        };
        let staking_contract = AccountInfo {
            code: Some(Bytecode::new_raw(staking_bytes.into())),
            ..Default::default()
        };
        let registry_contract = AccountInfo {
            code: Some(Bytecode::new_raw(registry_bytes.into())),
            ..Default::default()
        };
        let epoch_contract = AccountInfo {
            code: Some(Bytecode::new_raw(epoch_bytes.into())),
            ..Default::default()
        };
        let hello_contract = AccountInfo {
            code: Some(Bytecode::new_raw(hello_bytes.into())),
            ..Default::default()
        };
        let rep_scores_contract = AccountInfo {
            code: Some(Bytecode::new_raw(rep_bytes.into())),
            ..Default::default()
        };
        let rewards_contract = AccountInfo {
            code: Some(Bytecode::new_raw(rewards_bytes.into())),
            ..Default::default()
        };
        let rewards_agg_contract = AccountInfo {
            code: Some(Bytecode::new_raw(rewards_agg_bytes.into())),
            ..Default::default()
        };

        // Insert into db.
        state
            .db
            .insert_account_info(token_address.to_fixed_bytes().into(), token_contract);
        state
            .db
            .insert_account_info(staking_address.to_fixed_bytes().into(), staking_contract);
        state
            .db
            .insert_account_info(registry_address.to_fixed_bytes().into(), registry_contract);
        state
            .db
            .insert_account_info(epoch_address.to_fixed_bytes().into(), epoch_contract);
        state
            .db
            .insert_account_info(genesis.hello.address.parse().unwrap(), hello_contract);
        state
            .db
            .insert_account_info(rep_address.to_fixed_bytes().into(), rep_scores_contract);
        state
            .db
            .insert_account_info(rewards_address.to_fixed_bytes().into(), rewards_contract);
        state.db.insert_account_info(
            rewards_agg_address.to_fixed_bytes().into(),
            rewards_agg_contract,
        );

        // Call the init transactions.
        let registry_tx = TransactionRequest {
            to: Some(registry_address.into()),
            data: genesis.registry.init_params,
            ..Default::default()
        };
        let epoch_tx = TransactionRequest {
            to: Some(epoch_address.into()),
            data: genesis.epoch.init_params,
            ..Default::default()
        };

        // Submit and commit the init txns to state.
        let _registry_res = state.execute(registry_tx, false).await.unwrap();
        let _epoch_res = state.execute(epoch_tx, false).await.unwrap();

        drop(state);

        self.commit(RequestCommit {}).await;

        ResponseInitChain::default()
    }

    #[tracing::instrument(skip(self))]
    async fn begin_block(&self, _begin_block_request: RequestBeginBlock) -> ResponseBeginBlock {
        ResponseBeginBlock::default()
    }

    #[tracing::instrument(skip(self))]
    async fn deliver_tx(&self, deliver_tx_request: RequestDeliverTx) -> ResponseDeliverTx {
        tracing::trace!("delivering tx");
        let mut state = self.current_state.lock().await;

        let mut tx: TransactionRequest = match serde_json::from_slice(&deliver_tx_request.tx) {
            Ok(tx) => tx,
            Err(_) => {
                tracing::error!("could not decode request");
                return ResponseDeliverTx {
                    data: "could not decode request".into(),
                    ..Default::default()
                };
            }
        };

        let mut to_epoch_contract: bool = false;
        // Resolve the `to`.
        match tx.to {
            Some(NameOrAddress::Address(addr)) => {
                if addr == EPOCH_ADDRESS.parse::<Address>().unwrap() {
                    to_epoch_contract = true;
                }
                tx.to = Some(addr.into())
            }
            None => (),
            _ => panic!("not an address"),
        };

        let result = state.execute(tx, false).await.unwrap();
        tracing::trace!("executed tx");

        if to_epoch_contract {
            if let ExecutionResult::Success {
                output: Output::Call(bytes),
                ..
            } = &result
            {
                let results = SignalEpochChangeReturn::decode(bytes)
                    .unwrap_or(SignalEpochChangeReturn(false));

                if results.0 {
                    return ResponseDeliverTx {
                        data: serde_json::to_vec(&ExecutionResponse::ChangeEpoch).unwrap(),
                        ..Default::default()
                    };
                }
            }
        }
        ResponseDeliverTx {
            data: serde_json::to_vec(&ExecutionResponse::Transaction).unwrap(),
            ..Default::default()
        }
    }

    #[tracing::instrument(skip(self))]
    async fn end_block(&self, end_block_request: RequestEndBlock) -> ResponseEndBlock {
        tracing::trace!("ending block");
        let mut current_state = self.current_state.lock().await;
        current_state.block_height = end_block_request.height;
        current_state.app_hash = vec![];
        tracing::trace!("done");

        ResponseEndBlock::default()
    }

    #[tracing::instrument(skip(self))]
    async fn commit(&self, _commit_request: RequestCommit) -> ResponseCommit {
        tracing::trace!("taking lock");
        let current_state = self.current_state.lock().await.clone();
        let mut committed_state = self.committed_state.lock().await;
        *committed_state = current_state;
        tracing::trace!("committed");

        ResponseCommit {
            data: vec![], // (*committed_state).app_hash.clone(),
            retain_height: 0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Mempool;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum ExecutionResponse {
    ChangeEpoch,
    Transaction,
}

#[async_trait]
impl MempoolTrait for Mempool {
    async fn check_tx(&self, _check_tx_request: RequestCheckTx) -> ResponseCheckTx {
        ResponseCheckTx::default()
    }
}

#[derive(Debug, Clone)]
pub struct Info<Db> {
    pub state: Arc<Mutex<State<Db>>>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Query {
    EthCall(TransactionRequest),
    Balance(Address),
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum QueryResponse {
    Tx(ExecutionResult),
    Balance(U256),
}

impl QueryResponse {
    pub fn as_tx(&self) -> &ExecutionResult {
        match self {
            QueryResponse::Tx(inner) => inner,
            _ => panic!("not a tx"),
        }
    }

    pub fn as_balance(&self) -> U256 {
        match self {
            QueryResponse::Balance(inner) => *inner,
            _ => panic!("not a balance"),
        }
    }
}

#[async_trait]
impl<Db: Send + Sync + Database + DatabaseCommit> InfoTrait for Info<Db> {
    async fn info(&self, _info_request: RequestInfo) -> ResponseInfo {
        let state = self.state.lock().await;

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
        let mut state = self.state.lock().await;

        let query: Query = match serde_json::from_slice(&query_request.data) {
            Ok(tx) => tx,
            // no-op just logger
            Err(_) => {
                return ResponseQuery {
                    value: "could not decode request".into(),
                    ..Default::default()
                };
            }
        };

        let resp = match query {
            Query::EthCall(tx) => {
                let result = state.execute(tx, true).await.unwrap();

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
            Query::Balance(address) => match state.db.basic(address.to_fixed_bytes().into()) {
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

#[derive(Debug, Clone, Default)]
pub struct Snapshot;

impl SnapshotTrait for Snapshot {}
