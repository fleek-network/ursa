use anyhow::{bail, Result};
use ethers::prelude::NameOrAddress;
use ethers::types::{Address, TransactionRequest};
use parking_lot::Mutex;
use revm::db::DatabaseRef;
use revm::primitives::{AccountInfo, CreateScheme, TransactTo, B160, U256};
use revm::{
    self,
    db::{CacheDB, EmptyDB},
    primitives::{Env, ExecutionResult, TxEnv},
    Database, DatabaseCommit,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
    pub(crate) fn execute(
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

#[derive(Clone)]
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

#[derive(Debug, Clone, Default)]
pub struct Mempool;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum ExecutionResponse {
    ChangeEpoch,
    Transaction,
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

#[derive(Serialize, Deserialize)]
pub enum ApplicationError {
    UnableToDecodeRequest,
    InvalidAddress,
    ExecutionError,
}
