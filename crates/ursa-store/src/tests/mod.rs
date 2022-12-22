use db::MemoryDB;
use simple_logger::SimpleLogger;
use std::sync::Arc;
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

pub fn get_store() -> Arc<UrsaStore<MemoryDB>> {
    let db = Arc::new(MemoryDB::default());
    Arc::new(UrsaStore::new(Arc::clone(&db)))
}