#[cfg(test)]
mod tests {
    use tokio::{task, sync::oneshot};
    use tracing::error;

    use crate::{tests::provider_engine_init, engine::ProviderCommand};

    #[tokio::test]
    async fn test_events() -> Result<(), Box<dyn std::error::Error>> {
        let (provider_engine, ..) = provider_engine_init()?;

        let (sender, receiver) = oneshot::channel();
        let msg = ProviderCommand::Put {
            context_id: b"some test root cid".to_vec(),
            sender,
        };
        let provider_sender = provider_engine.command_sender();

        task::spawn(async move {
            if let Err(err) = provider_engine.start().await {
                error!("[provider_task] - {:?}", err);
            }
        });

        let _ = provider_sender.send(msg);
        let _res = receiver.await?;

        Ok(())
    }
}
