#[cfg(test)]
mod tests {
    use super::*;
    use crate::{advertisement::Advertisement, provider::ProviderInterface};
    use db::MemoryDB;
    use libp2p::PeerId;
    use multihash::{Code, MultihashDigest};
    use simple_logger::SimpleLogger;
    use surf::Error as SurfError;
    use tokio::task;
    use tracing::log::LevelFilter;
    use ursa_network::{NetworkConfig, UrsaService};

    fn setup_logger(level: LevelFilter) {
        if let Err(_) = SimpleLogger::new()
            .with_level(level)
            .with_utc_timestamps()
            .init()
        {
            info!("Logger already set. Ignore.")
        }
    }

    fn get_store() -> Arc<Store<MemoryDB>> {
        let db = Arc::new(MemoryDB::default());
        Arc::new(Store::new(Arc::clone(&db)))
    }

    fn provider_engine_init() -> (ProviderEngine<MemoryDB>, PeerId) {
        setup_logger(LevelFilter::Debug);
        let mut config = ProviderConfig::default();
        config.port = 0;

        let network_config = NetworkConfig::default();
        let keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());

        let store = get_store();
        let index_store = get_store();

        let service = UrsaService::new(keypair.clone(), &network_config, Arc::clone(&store));

        let provider_engine = ProviderEngine::new(
            keypair.clone(),
            store,
            index_store,
            config.clone(),
            service.command_sender(),
        );
        (provider_engine, peer_id)
    }

    #[tokio::test]
    async fn test_create_ad() -> Result<(), Box<dyn std::error::Error>> {
        let (provider_engine, peer_id) = provider_engine_init();

        let mut provider_interface = provider_engine.provider.clone();

        task::spawn(async move {
            if let Err(err) = provider_engine.start().await {
                error!("[provider_task] - {:?}", err);
            }
        });

        let _ = task::spawn(async move {
            let ad = Advertisement {
                PreviousID: None,
                Provider: peer_id.to_base58(),
                Addresses: vec!["/ip4/127.0.0.1/tcp/6009".into()],
                Signature: Ipld::Bytes(vec![]),
                Entries: None,
                Metadata: Ipld::Bytes(vec![]),
                ContextID: Ipld::Bytes("ursa".into()),
                IsRm: false,
            };

            let id = provider_interface.create(ad).unwrap();

            let mut entries: Vec<Ipld> = vec![];
            let count = 10;

            for i in 0..count {
                let b = Into::<i32>::into(i).to_ne_bytes();
                let mh = Code::Blake2b256.digest(&b);
                entries.push(Ipld::Bytes(mh.to_bytes()))
            }
            let bytes = forest_encoding::to_vec(&entries)?;
            provider_interface.add_chunk(bytes, id)?;
            provider_interface.publish(id)?;

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

    #[tokio::test]
    async fn test_events() -> Result<(), Box<dyn std::error::Error>> {
        let (provider_engine, ..) = provider_engine_init();

        let (sender, receiver) = oneshot::channel();
        let msg = ProviderCommand::Put {
            context_id: b"some test root cid".to_vec(),
            sender,
        };
        let provider_sender = provider_engine.command_sender.clone();

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
