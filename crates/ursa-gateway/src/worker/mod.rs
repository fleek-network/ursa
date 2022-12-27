use std::sync::Arc;

use tokio::{
    select,
    sync::{mpsc::UnboundedReceiver, RwLock},
    task,
};
use tracing::{error, info};

use crate::cache::Tlrfu;

pub async fn start(mut receiver: UnboundedReceiver<String>, cache: Arc<RwLock<Tlrfu>>) {
    loop {
        let cache = Arc::clone(&cache);
        select! {
            Some(cid) = receiver.recv() => {
                info!("dispatch: {cid:?}");
                task::spawn(async move {
                    cache.write().await._insert(cid, vec![]).await.unwrap();
                });
            }
            else => {
                error!("gateway stopped: please check error log.");
                break;
            }
        }
    }
}
