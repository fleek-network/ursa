#[cfg(test)]
mod tests {
    use crate::{
        advertisement::Advertisement, provider::ProviderInterface, signed_head::SignedHead,
        tests::provider_engine_init,
    };

    use anyhow::Error;
    use forest_ipld::Ipld;
    use multihash::{Code, MultihashDigest};
    use surf::Error as SurfError;
    use tokio::task;
    use tracing::error;

    #[tokio::test]
    async fn test_create_ad() -> Result<(), Box<dyn std::error::Error>> {
        let (provider_engine, _, peer_id) = provider_engine_init(8070)?;
        let mut provider_interface = provider_engine.provider();

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
                .map_err(SurfError::into_inner)?;
            assert_eq!(signed_head.open()?.1, provider_interface.head().unwrap());
            Ok::<_, Error>(())
        })
        .await?;

        Ok(())
    }
}
