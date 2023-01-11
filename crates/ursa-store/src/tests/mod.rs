use std::sync::Arc;

use db::MemoryDB;
use simple_logger::SimpleLogger;
use tracing::{log::LevelFilter, warn};

use crate::UrsaStore;

pub fn setup_logger() {
    let level = LevelFilter::Debug;
    if let Err(err) = SimpleLogger::new()
        .with_level(level)
        .with_utc_timestamps()
        .init()
    {
        warn!("Logger already set {:?}:", err)
    }
}

pub fn get_store() -> UrsaStore<MemoryDB> {
    UrsaStore::new(Arc::new(MemoryDB::default()))
}
