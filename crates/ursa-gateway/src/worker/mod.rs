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
use tracing::{error, info, info_span, warn, Instrument};
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
                                info!("Process GetSyncAnnounce command with key: {key:?}");
                                if let Err(e) = cache.write().await.get(&key).await {
                                    error!("Process GetSyncAnnounce command error with key: {key:?} {e:?}");
                                    signal_tx.send(()).await.expect("Send signal successfully");
                                };
                            }.instrument(span));
                        },
                        CacheCommand::InsertSync{ key, value, ctx } => {
                            let span = info_span!("[Worker]: InsertSync");
                            span.set_parent(ctx);
                            spawn(async move {
                                info!("Process InsertSyncAnnounce command with key: {key:?}");
                                if let Err(e) = cache.write().await.insert(String::from(&key), value).await {
                                    error!("Process InsertSyncAnnounce command error with key: {key:?} {e:?}");
                                    signal_tx.send(()).await.expect("Send signal successfully");
                                };
                            }.instrument(span));
                        },
                        CacheCommand::Fetch{ cid, sender, ctx } => {
                            let span = info_span!("[Worker]: Fetch");
                            span.set_parent(ctx);
                            spawn(async move {
                                info!("Process FetchAnnounce command with cid: {cid:?}");
                                if let Err(e) = sender.send(resolver.resolve_content(&cid).await) {
                                    warn!("Process FetchAnnounce command error with cid: {cid:?}. Receiver stopped\n{e:?}");
                                }
                            }.instrument(span));
                        },
                        CacheCommand::TtlCleanUp => {
                            spawn(async move {
                                let span = info_span!("[Worker]: TtlCleanUp");
                                info!("Process TtlCleanUp command");
                                if let Err(e) = cache.write().await.ttl_cleanup().instrument(span).await {
                                    error!("Process TtlCleanUp command error {e:?}");
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
