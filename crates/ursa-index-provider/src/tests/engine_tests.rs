#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

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
        let (provider_engine, ursa_service, ..) = provider_engine_init(8072)?;
        let provider_sender = provider_engine.command_sender();
        let provider_interface = provider_engine.provider();

        let file = File::open("../../test_files/test.car".to_string()).await?;
        let size = file.metadata().await?.len();
        let reader = BufReader::new(file);
        let cids = load_car(provider_engine.store().blockstore(), reader).await?;

        info!("The inserted cids are: {cids:?}");

        let (sender, receiver) = oneshot::channel();
        let message = ProviderCommand::Put {
            context_id: cids[0].to_bytes(),
            size,
            sender,
        };

        task::spawn(async move {
            if let Err(err) = ursa_service.start().await {
                error!("[ursa_service] - {:?}", err);
            }
        });

        task::spawn(async move {
            if let Err(err) = provider_engine.start().await {
                error!("[provider_engine] - {:?}", err);
            }
        });
        thread::sleep(Duration::from_millis(2000));

        let _ = provider_sender.send(message);
        let _res = receiver.await?;

        let _ = task::spawn(async move {
            let signed_head: SignedHead = surf::get("http://0.0.0.0:8072/head")
                .recv_json()
                .await
                .map_err(SurfError::into_inner)?;
            assert_eq!(signed_head.open()?.1, provider_interface.head().unwrap());
            Ok::<_, Error>(())
        })
        .await?;
        Ok(())
    }
}
