#[cfg(test)]
mod tests {
    use crate::{
        api::NodeNetworkInterface,
        config::ServerConfig,
        server::Server,
        tests::{init, setup_logger},
    };
    use std::sync::Arc;
    use tracing::log::LevelFilter;

    #[tokio::test]
    async fn test_rpc_start() -> anyhow::Result<()> {
        setup_logger(LevelFilter::Info);
        let (ursa_service, provider_engine, store) = init()?;

        let interface = Arc::new(NodeNetworkInterface {
            store,
            network_send: ursa_service.command_sender(),
            provider_send: provider_engine.command_sender(),
        });

        let rpc = Server::new(interface);
        let _ = rpc.start(ServerConfig::default()).await;
        // TODO: test server start! call the http and rpc endpoints?
        Ok(())
    }
}
