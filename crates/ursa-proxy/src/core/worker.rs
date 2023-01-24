use std::fmt::Debug;
use tokio::{spawn, sync::mpsc::UnboundedReceiver, task::JoinHandle};
use tracing::info;

pub async fn start<T: Debug + Send + 'static>(
    mut worker_rx: UnboundedReceiver<T>,
) -> JoinHandle<()> {
    spawn(async move {
        while let Some(cmd) = worker_rx.recv().await {
            info!("[Worker] Received command {cmd:?}")
        }
    })
}
