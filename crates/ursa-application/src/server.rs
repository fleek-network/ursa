use crate::App;
use crate::ApplicationConfig;
use abci::async_api::Server;
use abci::Address;
use anyhow::{anyhow, Result};
use resolve_path::PathResolveExt;
use tokio::fs;

pub async fn application_start(config: ApplicationConfig) -> Result<()> {
    let ApplicationConfig { abci_uds } = config;

    let App {
        consensus,
        mempool,
        info,
        snapshot,
    } = App::new();

    let server = Server::new(consensus, mempool, info, snapshot);

    // Delete old socket if neccasary
    if abci_uds.exists() {
        fs::remove_file(&abci_uds).await?;
    }

    server
        .run(Address::from(abci_uds.resolve().to_path_buf()))
        .await
        .map_err(|e| anyhow!("Abci Err encountered an err: {:?}", e))?;
    Ok(())
}
