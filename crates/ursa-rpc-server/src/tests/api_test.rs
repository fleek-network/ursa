#[cfg(test)]
mod tests {
    use crate::{
        api::{NetworkInterface, NodeNetworkInterface},
        tests::{init, setup_logger},
    };
    use std::sync::Arc;
    use tokio::task;
    use tracing::{error, log::LevelFilter};

    #[ignore]
    #[tokio::test]
    async fn test_stream() -> anyhow::Result<()> {
        // TODO: fix this test case. running indefinitely
        setup_logger(LevelFilter::Info);
        let (ursa_service, provider_engine, store) = init()?;

        let network_send = ursa_service.command_sender();
        // Start libp2p service
        println!("hit here2");
        let service_task = task::spawn(async {
            if let Err(err) = ursa_service.start().await {
                error!("[service_task] - {:?}", err);
            }
        });

        let interface = Arc::new(NodeNetworkInterface {
            store,
            network_send,
            provider_send: provider_engine.command_sender(),
        });
        println!("hit here1");
        let cids = interface
            .put_file("../../test_files/test.car".to_string())
            .await?;
        interface.stream(cids[0]).await?;
        service_task.abort();
        Ok(())
    }
}
