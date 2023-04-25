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
use futures::executor;
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
use ursa_utils::evm::epoch_manager::{SignalEpochChangeReturn, EPOCH_ADDRESS};

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
            nonce: None,
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

                let _results = executor::block_on(state.execute(tx, false)).unwrap_or_else(|_| {
                    panic!(
                        "Invalid genesis.toml: Invalid init params on precompile {}",
                        contract.name
                    )
                });
            }
        });

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
                if addr == *EPOCH_ADDRESS {
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
