mod app;

pub use app::App;
use revm::{Database, DatabaseCommit};

mod config;
pub use config::ApplicationConfig;

mod server;
pub use server::application_start;

pub mod genesis;
pub mod types;

use crate::types::WithGenesisDb;
pub use types::{Consensus, ExecutionResponse, Info, Mempool, Snapshot, State};

pub trait AbciDb: Clone + Send + Sync + DatabaseCommit + Database + WithGenesisDb {}
impl<T: Clone + Send + Sync + DatabaseCommit + Database + WithGenesisDb> AbciDb for T {}
