pub mod cache;

use {
    crate::resolver::Resolver,
    cache::{WorkerCache, WorkerCacheCommand},
    std::sync::Arc,
    tokio::{
        select,
        sync::{mpsc::UnboundedReceiver, RwLock},
        task,
    },
    tracing::{error, info},
};

pub async fn start<Cache: WorkerCache>(
    mut rx: UnboundedReceiver<WorkerCacheCommand>,
    cache: Arc<RwLock<Cache>>,
    resolver: Arc<Resolver>,
) {
    loop {
        let cache = Arc::clone(&cache);
        let resolver = Arc::clone(&resolver);
        select! {
            Some(cmd) = rx.recv() => {
                match cmd {
                    WorkerCacheCommand::GetSync{key} => {
                        info!("Dispatch GetSyncAnnounce command with key: {key:?}");
                        task::spawn(async move {
                            if let Err(e) = cache.write().await.get(&key).await {
                                error!("Dispatch GetSyncAnnounce command error with key: {key:?} {e}");
                            }
                        });
                    },
                    WorkerCacheCommand::InsertSync{key, value} => {
                        info!("Dispatch InsertSyncAnnounce command with key: {key:?}");
                        task::spawn(async move {
                            if let Err(e) = cache.write().await.insert(String::from(&key), value).await {
                                error!("Dispatch InsertSyncAnnounce command error with key: {key:?} {e}");
                            };
                        });
                    },
                    WorkerCacheCommand::Fetch{cid, sender} => {
                        info!("Dispatch FetchAnnounce command with cid: {cid:?}");
                        task::spawn(async move {
                            let result = match resolver.provider_address_v4(&cid).await {
                                Ok(providers) => {
                                    match resolver.resolve_content(providers, &cid).await {
                                        Ok(content) => sender.send(Ok(Arc::new(content))),
                                        Err(message) => sender.send(Err(message))
                                    }
                                }
                                Err(message) => sender.send(Err(message))
                            };
                            if let Err(e) = result {
                                error!("Dispatch FetchAnnounce command error with cid: {cid:?} {e:?}");
                            }
                        });
                    },
                    WorkerCacheCommand::TtlCleanUp => {
                        info!("Dispatch TtlCleanUp command");
                        task::spawn(async move {
                            if let Err(e) = cache.write().await.ttl_cleanup().await {
                                error!("Dispatch TtlCleanUp command error {e}");
                            };
                        });
                    }
                }
            }
            else => {
                error!("Worker stopped: please check error log");
                break;
            }
        }
    }
}
