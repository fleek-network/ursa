use anyhow::anyhow;
use db::Store;
use fnv::FnvHashSet;
use fvm_ipld_blockstore::Blockstore;
use fvm_ipld_car::CarHeader;
use fvm_ipld_encoding::{de::DeserializeOwned, from_slice, ser::Serialize, to_vec, DAG_CBOR};
use integer_encoding::VarInt;
use ipld_traversal::blockstore::Blockstore as GSBlockstore;
use libipld::{
    cid,
    multihash::{Code, MultihashDigest},
    store::DefaultParams,
    Block, Cid, Result,
};
use libp2p_bitswap::BitswapStore;
use std::sync::Arc;

#[derive(Debug)]
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

    /// return the inner blockstore
    pub fn blockstore(&self) -> &S {
        &self.db
    }

    /// traverse a dag and get full dag given a root cid
    pub fn dag_traversal(&self, root_cid: &Cid) -> Result<Vec<(Cid, Vec<u8>)>> {
        let mut res = Vec::new();
        // get full dag starting with root id
        let mut current = FnvHashSet::default();
        let mut refs = FnvHashSet::default();
        current.insert(*root_cid);

        while let Some(cid) = current.iter().next().copied() {
            current.remove(&cid);
            if refs.contains(&cid) {
                continue;
            }
            match self.db.get(&cid)? {
                Some(data) => {
                    res.push((cid, data.clone()));
                    let next_block = Block::<DefaultParams>::new(cid, data)?;
                    next_block.references(&mut current)?;
                    refs.insert(cid);
                }
                None => {
                    // TODO: handle the case where parts of the dags are missing
                    return Err(anyhow!(
                        "The block with cid {:?} from the dag with the root {:?} is missing ",
                        cid,
                        root_cid
                    ));
                }
            }
        }
        Ok(res)
    }

    /// Calculate a car file size from a root cid
    pub fn car_size(&self, root_cid: &Cid) -> Result<u64> {
        let dag = self.dag_traversal(root_cid)?;

        let header_bytes = to_vec(&CarHeader {
            roots: vec![*root_cid],
            version: 1,
        })?;
        let mut len = header_bytes.len();

        for (cid, bytes) in dag {
            let block_len = bytes.len() + cid.to_bytes().len();
            len += block_len.encode_var_vec().len(); // varint size
            len += block_len;
        }

        Ok(len as u64)
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

#[derive(Clone, Debug)]
pub struct GraphSyncStorage<P>(pub Arc<UrsaStore<P>>)
where
    P: Blockstore + Store + Send + Sync + 'static;

impl<S> GraphSyncStorage<S>
where
    S: Blockstore + Store + Send + Sync + 'static,
{
    pub fn insert(&mut self, block: &Block<DefaultParams>) -> Result<()> {
        self.0.db.put_keyed(block.cid(), block.data()).unwrap();

        Ok(())
    }

    pub fn get(&mut self, cid: &Cid) -> Result<Option<Vec<u8>>> {
        Ok(self.0.db.get(cid).unwrap())
    }
}

impl<S> GSBlockstore for GraphSyncStorage<S>
where
    S: Blockstore + Store + Send + Sync + 'static,
{
    fn get(&self, k: &cid::Cid) -> Result<Option<Vec<u8>>> {
        self.0.blockstore().get(k)
    }

    fn put_keyed(&self, k: &Cid, block: &[u8]) -> Result<()> {
        self.0.blockstore().put_keyed(k, block)
    }

    fn delete_block(&self, k: &Cid) -> Result<()> {
        self.0
            .blockstore()
            .delete(k.to_bytes())
            .map_err(|e| e.into())
    }
}

#[cfg(test)]
#[path = "tests/store_tests.rs"]
mod store_tests;
