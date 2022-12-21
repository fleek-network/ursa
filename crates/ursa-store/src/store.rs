use anyhow::anyhow;
use cid::{
    multihash::{Code, MultihashDigest},
    Cid,
};
use db::Store;
use fnv::FnvHashSet;
use fvm_ipld_blockstore::Blockstore;
use fvm_ipld_encoding::{de::DeserializeOwned, from_slice, ser::Serialize, to_vec, DAG_CBOR};
use libipld::store::DefaultParams;
use libipld::{Block, Result};
use libp2p_bitswap::BitswapStore;
use std::sync::Arc;
use ursa_utils::convert_cid;

pub struct UrsaStore<S> {
    pub db: Arc<S>,
}

impl<S> UrsaStore<S>
where
    S: Blockstore + Store + Send + Sync + 'static,
{
    pub fn new(db: Arc<S>) -> Self {
        Self { db }
    }

    pub fn blockstore(&self) -> &S {
        &self.db
    }
}

/// Extension methods for inserting and retrieving IPLD data with CIDs
pub trait BlockstoreExt: Blockstore {
    /// Get typed object from block store by CID
    fn get_obj<T>(&self, cid: &Cid) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        match self.get(cid)? {
            Some(bz) => Ok(Some(from_slice(&bz)?)),
            None => Ok(None),
        }
    }

    /// Put an object in the block store and return the Cid identifier.
    fn put_obj<S>(&self, obj: &S, code: Code) -> Result<Cid>
    where
        S: Serialize,
    {
        let bytes = to_vec(obj)?;
        self.put_raw(bytes, code)
    }

    /// Put raw bytes in the block store and return the Cid identifier.
    fn put_raw(&self, bytes: Vec<u8>, code: Code) -> Result<Cid> {
        let cid = Cid::new_v1(DAG_CBOR, code.digest(&bytes));
        self.put_keyed(&cid, &bytes)?;
        Ok(cid)
    }

    /// Batch put CBOR objects into block store and returns vector of CIDs
    fn bulk_put<'a, S, V>(&self, values: V, code: Code) -> Result<Vec<Cid>>
    where
        Self: Sized,
        S: Serialize + 'a,
        V: IntoIterator<Item = &'a S>,
    {
        let keyed_objects = values
            .into_iter()
            .map(|value| {
                let bytes = to_vec(value)?;
                let cid = Cid::new_v1(DAG_CBOR, code.digest(&bytes));
                Ok((cid, bytes))
            })
            .collect::<Result<Vec<_>>>()?;

        let cids = keyed_objects
            .iter()
            .map(|(cid, _)| cid.to_owned())
            .collect();

        self.put_many_keyed(keyed_objects)?;

        Ok(cids)
    }
}

impl<T: Blockstore> BlockstoreExt for T {}

pub struct BitswapStorage<P>(pub Arc<UrsaStore<P>>)
where
    P: Blockstore + Store + Send + Sync + 'static;

impl<P> BitswapStore for BitswapStorage<P>
where
    P: Blockstore + Store + Send + Sync + 'static,
{
    type Params = DefaultParams;

    fn contains(&mut self, cid: &Cid) -> Result<bool> {
        self.0.db.has(cid)
    }

    fn get(&mut self, cid: &Cid) -> Result<Option<Vec<u8>>> {
        Ok(self.0.db.get(cid).unwrap())
    }

    fn insert(&mut self, block: &Block<Self::Params>) -> Result<()> {
        self.0.db.put_keyed(block.cid(), block.data()).unwrap();

        Ok(())
    }

    fn missing_blocks(&mut self, cid: &Cid) -> Result<Vec<Cid>> {
        let mut stack = vec![*cid];
        let mut missing = vec![];

        while let Some(cid) = stack.pop() {
            if let Some(data) = self.get(&cid)? {
                let block = Block::<Self::Params>::new_unchecked(cid, data);
                block.references(&mut stack)?;
            } else {
                missing.push(cid);
            }
        }

        Ok(missing)
    }
}

pub trait Dag {
    /// traverse a dag and get full dag given a root cid
    fn dag_traversal(&self, root_cid: &Cid) -> Result<Vec<(Cid, Vec<u8>)>>;
}

impl<S> Dag for UrsaStore<S>
where
    S: Blockstore + Sync + Send + 'static,
{
    fn dag_traversal(&self, root_cid: &Cid) -> Result<Vec<(Cid, Vec<u8>)>> {
        let mut res = Vec::new();
        // get full dag starting with root id
        let mut current = FnvHashSet::default();
        let mut refs = FnvHashSet::default();
        current.insert(convert_cid::<Cid>(root_cid.to_bytes()));

        while let Some(cid) = current.iter().next().copied() {
            current.remove(&cid);
            if refs.contains(&cid) {
                continue;
            }
            match self.db.get(&convert_cid(cid.to_bytes()))? {
                Some(data) => {
                    res.push((convert_cid(cid.to_bytes()), data.clone()));
                    let next_block = Block::<DefaultParams>::new(cid, data)?;
                    next_block.references(&mut current)?;
                    refs.insert(cid);
                }
                None => {
                    // TODO: handle the case where parts of the dags are missing
                    return Err(anyhow!(
                        "The block wiht cid {:?} from the dag with the root {:?} is missing ",
                        cid,
                        root_cid
                    ));
                }
            }
        }
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    use db::{rocks::RocksDb, rocks_config::RocksDbConfig};
    use simple_logger::SimpleLogger;

    #[tokio::test]
    async fn get_missing_blocks() {
        SimpleLogger::new().with_utc_timestamps().init().unwrap();

        let db1 = Arc::new(
            RocksDb::open("ursa_db", &RocksDbConfig::default())
                .expect("Opening RocksDB must succeed"),
        );

        let store1 = Arc::new(UrsaStore::new(Arc::clone(&db1)));
        let mut bitswap_store_1 = BitswapStorage(store1);

        let cid =
            Cid::from_str("bafybeihybv5apjuvkpaw62l34ui7t363pt3hwxbz7rltrpjklvzrbviq5m").unwrap();

        if let Ok(res) = bitswap_store_1.missing_blocks(&convert_cid(cid.to_bytes())) {
            println!("vec of missing blocks: {res:?}");
        }
    }
}
