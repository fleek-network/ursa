use crate::cache::CacheWorker;
use std::fmt::Debug;
use tokio::{select, spawn, sync::mpsc::UnboundedReceiver, task::JoinHandle};
use tracing::info;

pub async fn start<Cmd: Debug + Send + 'static, C: CacheWorker + CacheWorker<Command = Cmd>>(
    mut worker_rx: UnboundedReceiver<Cmd>,
    cache: C,
) -> JoinHandle<()> {
    spawn(async move {
        loop {
            select! {
                Some(cmd) = worker_rx.recv() => {
                     info!("[Worker] Received command {cmd:?}");
                    // TODO: Handle error.
                    let mut cache = cache.clone();
                    spawn(async move {
                        cache.handle_command(cmd).await
                    });
                }
            }
        }
    })
}
