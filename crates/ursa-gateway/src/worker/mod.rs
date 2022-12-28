pub mod cache;

use std::sync::Arc;

use cache::{WorkerCache, WorkerCacheCommand};
use serde_json::to_vec;
use tokio::{
    select,
    sync::{mpsc::UnboundedReceiver, RwLock},
    task,
};
use tracing::{error, info};

use crate::indexer::Indexer;

pub async fn start<Cache: WorkerCache>(
    mut rx: UnboundedReceiver<WorkerCacheCommand>,
    cache: Arc<RwLock<Cache>>,
    indexer: Arc<Indexer>,
) {
    loop {
        let cache = Arc::clone(&cache);
        let indexer = Arc::clone(&indexer);
        select! {
            Some(cmd) = rx.recv() => {
                match cmd {
                    WorkerCacheCommand::GetSync{key} => {
                        info!("Dispatch GetSyncAnnounce command with key: {key:?}");
                        task::spawn(async move {
                            if let Err(e) = cache.write().await.get(&key).await {
                                error!("Dispatch GetSyncAnnounce command error with key: {key:?}\n{e}");
                            }
                        });
                    },
                    WorkerCacheCommand::InsertSync{key, value} => {
                        info!("Dispatch InsertSyncAnnounce command with key: {key:?}");
                        task::spawn(async move {
                            if let Err(e) = cache.write().await.insert(String::from(&key), value).await {
                                error!("Dispatch InsertSyncAnnounce command error with key: {key:?}\n{e}");
                            };
                        });
                    },
                    WorkerCacheCommand::Fetch{cid, sender} => {
                        info!("Dispatch FetchAnnounce command with cid: {cid:?}");
                        task::spawn(async move {
                            let result = match indexer.query(String::from(&cid)).await {
                                Ok(provider_result) => {
                                    // TODO: query cache node
                                    sender.send(Ok(Arc::new(to_vec(&provider_result).unwrap())))
                                },
                                Err(message) => sender.send(Err(message))
                            };
                            if let Err(e) = result {
                                error!("Dispatch FetchAnnounce command error with cid: {cid:?}\n{e:?}");
                            }
                        });
                    }
                }
            }
            else => {
                error!("Worker stopped: please check error log.");
                break;
            }
        }
    }
}
