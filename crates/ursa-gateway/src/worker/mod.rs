use std::sync::Arc;

use tokio::{
    select,
    sync::{mpsc::UnboundedReceiver, RwLock},
    task,
};
use tracing::{error, info};

use crate::{
    cache::{Cache, CacheCmd},
    indexer::Indexer,
};
use serde_json::to_vec;

pub async fn start(
    mut rx: UnboundedReceiver<CacheCmd>,
    cache: Arc<RwLock<Cache>>,
    indexer: Indexer,
) {
    let indexer = Arc::new(indexer);
    loop {
        let cache = Arc::clone(&cache);
        let indexer = Arc::clone(&indexer);
        select! {
            Some(cmd) = rx.recv() => {
                match cmd {
                    CacheCmd::GetSync{key} => {
                        info!("Dispatch GetSyncAnnounce command with key: {key:?}");
                        task::spawn(async move {
                            if let Err(e) = cache.write().await.get(&key).await {
                                error!("Dispatch GetSyncAnnounce command error with key: {key:?}\n{e}");
                            }
                        });
                    },
                    CacheCmd::InsertSync{key, value} => {
                        info!("Dispatch InsertSyncAnnounce command with key: {key:?}");
                        task::spawn(async move {
                            if let Err(e) = cache.write().await.insert(String::from(&key), value).await {
                                error!("Dispatch InsertSyncAnnounce command error with key: {key:?}\n{e}");
                            };
                        });
                    },
                    CacheCmd::Fetch{cid, sender} => {
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
                error!("Gateway stopped: please check error log.");
                break;
            }
        }
    }
}
