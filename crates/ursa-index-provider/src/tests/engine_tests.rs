#[cfg(test)]
mod tests {
    use anyhow::Error;
    use async_fs::File;
    use futures::io::BufReader;
    use fvm_ipld_car::load_car;
    use surf::Error as SurfError;
    use tokio::{sync::oneshot, task};
    use tracing::{error, info};

    use crate::{engine::ProviderCommand, signed_head::SignedHead, tests::provider_engine_init};

    #[tokio::test]
    async fn test_events() -> Result<(), Box<dyn std::error::Error>> {
        let (provider_engine, peer_id) = provider_engine_init()?;
        let provider_interface = provider_engine.provider();
        let provider_sender = provider_engine.command_sender();

        let file = File::open("../../test_files/test.car".to_string()).await?;
        let reader = BufReader::new(file);
        let cids = load_car(provider_interface.store().blockstore(), reader).await?;

        info!("The inserted cids are: {cids:?}");

        let (sender, receiver) = oneshot::channel();
        let message = ProviderCommand::Put {
            context_id: cids[0].to_bytes(),
            sender,
        };

        task::spawn(async move {
            if let Err(err) = provider_engine.start().await {
                error!("[provider_task] - {:?}", err);
            }
        });

        let _ = provider_sender.send(message);
        let _res = receiver.await?;

        let _ = task::spawn(async move {
            let signed_head: SignedHead = surf::get("http://0.0.0.0:8070/head")
                .recv_json()
                .await
                .map_err(|e| SurfError::into_inner(e))?;
            assert_eq!(signed_head.open()?.1, provider_interface.head().unwrap());
            Ok::<_, Error>(())
        })
        .await?;
        Ok(())
    }
}
