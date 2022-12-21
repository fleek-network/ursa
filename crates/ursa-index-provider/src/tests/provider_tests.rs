#[cfg(test)]
mod tests {
    use crate::{
        advertisement::Advertisement, provider::ProviderInterface, signed_head::SignedHead,
        tests::provider_engine_init,
    };

    use anyhow::Error;
    use cid::{multihash::{Code, MultihashDigest}, Cid};
    use libipld_core::ipld::Ipld;
    use surf::Error as SurfError;
    use tokio::task;
    use tracing::{error, info};

    #[tokio::test]
    async fn test_create_and_get_add() -> Result<(), Box<dyn std::error::Error>> {
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
            let published_ad = provider_interface.publish(id)?;

            let signed_head: SignedHead = surf::get("http://0.0.0.0:8070/head")
                .recv_json()
                .await
                .map_err(|e| SurfError::into_inner(e))?;
            let head_cid = signed_head.open()?.1.to_string();
            assert_eq!(head_cid, provider_interface.head().unwrap().to_string());
            info!("The head was verified");
            
            let data: Vec<u8> = surf::get(format!("http://0.0.0.0:8070/{head_cid}"))
                .recv_bytes()
                .await
                .map_err(|e| SurfError::into_inner(e))?;

            let ad: Advertisement = forest_encoding::from_slice(&data)?;
            let mut ad_entries = forest_encoding::to_vec(&ad.clone().Entries.unwrap())?;
            ad_entries.drain(0..3);
            let entries_cid = Cid::try_from(ad_entries)?;
            let ad_link = Ipld::Link(entries_cid.into());

            let new_ad = Advertisement {
                Entries: Some(ad_link),
                ..ad
            };
            assert_eq!(new_ad, published_ad);

            Ok::<_, Error>(())
        })
        .await?;

        Ok(())
    }
}
