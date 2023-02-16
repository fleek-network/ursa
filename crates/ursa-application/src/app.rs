use crate::types::{Consensus, Info, Mempool, Snapshot, State};
use revm::db::{CacheDB, EmptyDB};
use revm::primitives::AccountInfo;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct App<Db> {
    pub mempool: Mempool,
    pub snapshot: Snapshot,
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
        let mut state = State {
            db: CacheDB::new(EmptyDB()),
            block_height: Default::default(),
            app_hash: Default::default(),
            env: Default::default(),
        };

        state.db.insert_account_info(
            "0xDAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
                .parse()
                .unwrap(),
            AccountInfo {
                balance: ethers::utils::parse_ether(1.5).unwrap().into(),
                ..Default::default()
            },
        );

        let committed_state = Arc::new(Mutex::new(state.clone()));
        let current_state = Arc::new(Mutex::new(state));

        let consensus = Consensus {
            committed_state: committed_state.clone(),
            current_state,
        };

        let mempool = Mempool::default();
        let info = Info {
            state: committed_state,
        };
        let snapshot = Snapshot::default();

        App {
            consensus,
            mempool,
            info,
            snapshot,
        }
    }
}
