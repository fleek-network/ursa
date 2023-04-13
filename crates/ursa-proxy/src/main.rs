use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tokio::{
    select,
    signal::{
        ctrl_c,
        unix::{signal, SignalKind},
    },
    sync::{
        mpsc::{self, Sender},
        oneshot,
    },
    task::{self, JoinHandle},
};
use tracing::{error, info};
use ursa_proxy::{
    cache::moka_cache::MokaCache,
    cli::{Cli, Commands},
    config::load_config,
    core,
};

#[tokio::main]
async fn main() -> Result<()> {
    let Cli {
        command: Commands::Daemon(opts),
    } = Cli::parse();
    let config = load_config(&opts.config.parse::<PathBuf>()?)?;
    let moka_config = config.moka.clone().unwrap_or_default();
    let cache = MokaCache::new(moka_config);
    let (signal_shutdown_tx, signal_shutdown_rx) = mpsc::channel(1);
    let (proxy_error_tx, proxy_error_rx) = oneshot::channel();
    let proxy = task::spawn(async move {
        if let Err(e) = core::start(config, cache, signal_shutdown_rx).await {
            proxy_error_tx.send(e).expect("Sending to succeed");
        }
    });

    #[cfg(unix)]
    let terminate = async {
        signal(SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    select! {
        _ = terminate => {
            graceful_shutdown(proxy, signal_shutdown_tx).await;
        }
        _ = ctrl_c() => {
            graceful_shutdown(proxy, signal_shutdown_tx).await;
        }
        e = proxy_error_rx => {
            error!("Proxy error {e:?}");
            proxy.await.expect("Proxy to shut down successfully");
        }
    }
    info!("Proxy shut down successfully");
    Ok(())
}

async fn graceful_shutdown(proxy: JoinHandle<()>, shutdown_tx: Sender<()>) {
    shutdown_tx.send(()).await.expect("Sending to succeed");
    proxy.await.expect("Proxy to shut down successfully");
}
