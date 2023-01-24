#[cfg(test)]
mod tests {
    use async_fs::File;
    use futures::io::BufReader;
    use fvm_ipld_car::{load_car, CarReader};
    use libipld::Cid;
    use std::path::Path;
    use std::sync::Arc;

    use crate::tests::{get_store, setup_logger};

    #[tokio::test]
    async fn test_dag_traversal() -> anyhow::Result<()> {
        setup_logger();
        let store = get_store();
        let store_2 = Arc::clone(&store);

        let path = Path::new("../../test_files/test.car");
        let file = File::open(path).await?;
        let reader = BufReader::new(file);
        let cids = load_car(store.blockstore(), reader).await?;

        let file_h = File::open(path).await?;
        let reader_h = BufReader::new(file_h);
        let mut car_reader = CarReader::new(reader_h).await?;

        let mut cids_vec = Vec::<Cid>::new();
        while let Some(block) = car_reader.next_block().await? {
            cids_vec.push(block.cid);
        }

        let res = store_2.dag_traversal(&cids[0])?;
        assert_eq!(cids_vec.len(), res.len());
        // todo: check if they both have sam cids
        Ok(())
    }
}
