use crate::App;
use crate::ApplicationConfig;
use abci::async_api::Server;
use anyhow::Result;
use std::net::SocketAddr;

pub async fn application_start(config: ApplicationConfig) -> Result<()> {
    let ApplicationConfig { domain } = config;

    let App {
        consensus,
        mempool,
        info,
        snapshot,
    } = App::new();
    let server = Server::new(consensus, mempool, info, snapshot);

    let addr = domain.parse::<SocketAddr>().unwrap();

    server.run(addr).await?;
    Ok(())
}
