pub mod cache;

use std::sync::Arc;

use cache::{WorkerCache, WorkerCacheCommand};
use tokio::{
    select, spawn,
    sync::{
        broadcast::Receiver,
        mpsc::{Sender, UnboundedReceiver},
        RwLock,
    },
    task::JoinHandle,
};
use tracing::{error, info};

use crate::resolver::Resolver;

pub fn start<Cache: WorkerCache>(
    mut cache_worker_rx: UnboundedReceiver<WorkerCacheCommand>,
    cache: Arc<RwLock<Cache>>,
    resolver: Arc<Resolver>,
    signal_tx: Sender<()>,
    mut shutdown_rx: Receiver<()>,
) -> JoinHandle<()> {
    spawn(async move {
        loop {
            let signal_tx = signal_tx.clone(); // move to cache worker thread
            select! {
                Some(cmd) = cache_worker_rx.recv() => {
                    let cache = Arc::clone(&cache);
                    let resolver = Arc::clone(&resolver);
                    match cmd {
                        WorkerCacheCommand::GetSync{key} => {
                            info!("Dispatch GetSyncAnnounce command with key: {key:?}");
                            spawn(async move {
                                if let Err(e) = cache.write().await.get(&key).await {
                                    error!("Dispatch GetSyncAnnounce command error with key: {key:?} {e:?}");
                                    signal_tx.send(()).await.expect("Send signal successfully");
                                };
                            });
                        },
                        WorkerCacheCommand::InsertSync{key, value} => {
                            info!("Dispatch InsertSyncAnnounce command with key: {key:?}");
                            spawn(async move {
                                if let Err(e) = cache.write().await.insert(String::from(&key), value).await {
                                    error!("Dispatch InsertSyncAnnounce command error with key: {key:?} {e:?}");
                                    signal_tx.send(()).await.expect("Send signal successfully");
                                };
                            });
                        },
                        WorkerCacheCommand::Fetch{cid, sender} => {
                            info!("Dispatch FetchAnnounce command with cid: {cid:?}");
                            spawn(async move {
                                let result = match resolver.resolve_content(&cid).await {
                                    Ok(content) => sender.send(Ok(content)),
                                    Err(message) => sender.send(Err(message))
                                };
                                if let Err(e) = result {
                                    error!("Dispatch FetchAnnounce command error with cid: {cid:?} {e:?}");
                                    signal_tx.send(()).await.expect("Send signal successfully");
                                }
                            });
                        },
                        WorkerCacheCommand::TtlCleanUp => {
                            info!("Dispatch TtlCleanUp command");
                            spawn(async move {
                                if let Err(e) = cache.write().await.ttl_cleanup().await {
                                    error!("Dispatch TtlCleanUp command error {e:?}");
                                    signal_tx.send(()).await.expect("Send signal successfully");
                                };
                            });
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Worker stopped");
                    break;
                }
            }
        }
    })
}
