use crate::cache::CacheWorker;
use std::fmt::Debug;
use tokio::{spawn, sync::mpsc::UnboundedReceiver, task::JoinHandle};
use tracing::info;

pub async fn start<Cmd: Debug + Send + 'static, C: CacheWorker + CacheWorker<Command = Cmd>>(
    mut worker_rx: UnboundedReceiver<Cmd>,
    cache: C,
) -> JoinHandle<()> {
    // TODO: Implement safe shutdown.
    spawn(async move {
        loop {
            while let Some(cmd) = worker_rx.recv().await {
                info!("[Worker] Received command {cmd:?}");
                let mut cache = cache.clone();
                spawn(async move { cache.handle_command(cmd).await });
            }
        }
    })
}
