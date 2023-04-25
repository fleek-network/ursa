use crate::App;
use crate::ApplicationConfig;
use anyhow::{anyhow, Result};
use async_abci::ServerXX;
use resolve_path::PathResolveExt;

pub async fn application_start(config: ApplicationConfig) -> Result<()> {
    let ApplicationConfig { abci_uds } = config;

    let app = App::new();
    let server = ServerXX::new(app);
    println!("abci_uds: {:?}", abci_uds.clone());

    server
        .bind_unix(abci_uds.resolve())
        .await
        .map_err(|e| anyhow!("invalid abci_uds path: {:?}", e))?
        .run()
        .await
        .map_err(|e| anyhow!("Abci Err encountered an err: {:?}", e))?;
    Ok(())
}
