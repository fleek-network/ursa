pub mod cache;

use std::sync::Arc;

use cache::{worker::WorkerCache, CacheCommand};
use tokio::{
    select, spawn,
    sync::{
        mpsc::{Receiver, Sender, UnboundedReceiver},
        RwLock,
    },
    task::JoinHandle,
};
use tracing::{error, info, info_span, Instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::resolver::Resolver;

pub fn start<Cache: WorkerCache>(
    mut cache_worker_rx: UnboundedReceiver<CacheCommand>,
    cache: Arc<RwLock<Cache>>,
    resolver: Arc<Resolver>,
    signal_tx: Sender<()>,
    mut shutdown_rx: Receiver<()>,
) -> JoinHandle<()> {
    spawn(async move {
        info!("Main worker start");
        loop {
            let signal_tx = signal_tx.clone(); // move to cache worker thread
            select! {
                Some(cmd) = cache_worker_rx.recv() => {
                    let cache = Arc::clone(&cache);
                    let resolver = Arc::clone(&resolver);
                    match cmd {
                        CacheCommand::GetSync{ key, ctx } => {
                            let span = info_span!("[Worker]: GetSync");
                            span.set_parent(ctx);
                            spawn(async move {
                                info!("Dispatch GetSyncAnnounce command with key: {key:?}");
                                if let Err(e) = cache.write().await.get(&key).await {
                                    error!("Dispatch GetSyncAnnounce command error with key: {key:?} {e:?}");
                                    signal_tx.send(()).await.expect("Send signal successfully");
                                };
                            }.instrument(span));
                        },
                        CacheCommand::InsertSync{ key, value, ctx } => {
                            let span = info_span!("[Worker]: InsertSync");
                            span.set_parent(ctx);
                            spawn(async move {
                                info!("Dispatch InsertSyncAnnounce command with key: {key:?}");
                                if let Err(e) = cache.write().await.insert(String::from(&key), value).await {
                                    error!("Dispatch InsertSyncAnnounce command error with key: {key:?} {e:?}");
                                    signal_tx.send(()).await.expect("Send signal successfully");
                                };
                            }.instrument(span));
                        },
                        CacheCommand::Fetch{ cid, sender, ctx } => {
                            let span = info_span!("[Worker]: Fetch");
                            span.set_parent(ctx);
                            spawn(async move {
                                info!("Dispatch FetchAnnounce command with cid: {cid:?}");
                                let result = match resolver.resolve_content(&cid).await {
                                    Ok(content) => sender.send(Ok(content)),
                                    Err(message) => sender.send(Err(message))
                                };
                                if let Err(e) = result {
                                    error!("Dispatch FetchAnnounce command error with cid: {cid:?} {e:?}");
                                    signal_tx.send(()).await.expect("Send signal successfully");
                                }
                            }.instrument(span));
                        },
                        CacheCommand::TtlCleanUp => {
                            spawn(async move {
                                info!("Dispatch TtlCleanUp command");
                                if let Err(e) = cache.write().await.ttl_cleanup().await {
                                    error!("Dispatch TtlCleanUp command error {e:?}");
                                    signal_tx.send(()).await.expect("Send signal successfully");
                                };
                            });
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Main worker stopped");
                    break;
                }
            }
        }
    }.instrument(info_span!("Main worker")))
}
