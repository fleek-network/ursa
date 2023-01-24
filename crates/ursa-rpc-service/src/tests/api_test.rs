#[cfg(test)]
mod tests {
    use crate::api::{NetworkInterface, NodeNetworkInterface};
    use crate::config::OriginConfig;
    use crate::tests::{dummy_ipfs, init, setup_logger};
    use anyhow::Result;
    use async_fs::{remove_file, File};
    use futures::io::BufReader;
    use fvm_ipld_car::load_car;
    use std::path::Path;
    use std::sync::Arc;
    use tokio::task;
    use tracing::error;

    #[tokio::test]
    async fn test_put_and_get() -> Result<()> {
        setup_logger();
        let (mut ursa_service, mut provider_engine, store) = init()?;

        let interface = Arc::new(NodeNetworkInterface::new(
            Arc::clone(&store),
            ursa_service.command_sender(),
            provider_engine.command_sender(),
            Default::default(),
        ));

        // the test case does not start the provider engine, so the best way
        // for put_file to not call provider engine is to close the channel
        provider_engine.command_receiver().close();
        ursa_service.close_command_receiver();

        let put_file = interface
            .put_file("../../test_files/test.car".to_string())
            .await?;
        let root_cid = put_file[0];

        interface
            .get_file("../../test_files".to_string(), root_cid)
            .await?;

        let path = format!("../../test_files/{root_cid}.car");
        let path = Path::new(&path);
        let file = File::open(path).await?;
        let reader = BufReader::new(file);
        let cids = load_car(store.blockstore(), reader).await?;

        assert_eq!(cids[0], root_cid);
        remove_file(path).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_origin_fallback() -> Result<()> {
        setup_logger();
        task::spawn(async {
            if let Err(e) = dummy_ipfs().await {
                error!("dummy ipfs server failed: {}", e);
            }
        });

        const IPFS_CID: &str = "bafkreihwcrnsi2tqozwq22k4vl7flutu43jlxgb3tenewysm2xvfuej5i4";
        const IPFS_LEN: usize = 26849;

        let (node, mut provider, store) = init()?;
        let command_sender = node.command_sender();
        provider.command_receiver().close();
        tokio::task::spawn(async move {
            node.start().await.unwrap();
        });

        let interface = Arc::new(NodeNetworkInterface::new(
            Arc::clone(&store),
            command_sender,
            provider.command_sender(),
            OriginConfig {
                ipfs_gateway: "127.0.0.1:9682".to_string(),
                use_https: Some(false),
            },
        ));

        // since we have no peers, get will fallback to origin
        let (cid, data) = &interface.get_data(IPFS_CID.parse()?).await?[0];
        assert_eq!(cid.to_string(), IPFS_CID);
        assert_eq!(data.len(), IPFS_LEN);

        Ok(())
    }
}
