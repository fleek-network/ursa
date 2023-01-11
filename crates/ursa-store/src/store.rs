use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use anyhow::anyhow;
use cid::{
    multihash::{Code, MultihashDigest},
    Cid,
};
use db::Store;
use fnv::FnvHashSet;
use fvm_ipld_blockstore::Blockstore;
use fvm_ipld_encoding::{de::DeserializeOwned, from_slice, ser::Serialize, to_vec, DAG_CBOR};
use ipld_traversal::blockstore::Blockstore as GSBlockstore;
use libipld::{
    codec::References,
    store::{DefaultParams, StoreParams},
    Block, Ipld, Result,
};
use libp2p_bitswap::BitswapStore;

pub trait StoreBase: Blockstore + Store + Send + Sync + 'static {}
impl<T> StoreBase for T where T: Blockstore + Store + Send + Sync + 'static {}

pub struct UrsaStore<S> {
    pub db: Arc<S>,
}

impl<S> Clone for UrsaStore<S> {
    fn clone(&self) -> Self {
        UrsaStore {
            db: Arc::clone(&self.db),
        }
    }
}

impl<S> Debug for UrsaStore<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UrsaStore").finish()
    }
}

impl<S: StoreBase> UrsaStore<S> {
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

pub struct BitswapStorage<S, P = DefaultParams>(pub UrsaStore<S>, pub PhantomData<P>);

impl<S, P> From<UrsaStore<S>> for BitswapStorage<S, P> {
    fn from(value: UrsaStore<S>) -> Self {
        Self(value, PhantomData::default())
    }
}

impl<S: StoreBase, P: StoreParams> BitswapStore for BitswapStorage<S, P>
where
    Ipld: References<P::Codecs>,
{
    type Params = P;

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

impl<S: StoreBase> Dag for UrsaStore<S> {
    fn dag_traversal(&self, root_cid: &Cid) -> Result<Vec<(Cid, Vec<u8>)>> {
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

impl<S: StoreBase> GSBlockstore for UrsaStore<S> {
    fn get(&self, k: &cid::Cid) -> Result<Option<Vec<u8>>> {
        self.db.get(k)
    }

    fn put_keyed(&self, k: &Cid, block: &[u8]) -> Result<()> {
        self.db.put_keyed(k, block)
    }

    fn delete_block(&self, k: &Cid) -> Result<()> {
        self.db.delete(k.to_bytes()).map_err(|e| e.into())
    }
}

#[cfg(test)]
#[path = "tests/store_tests.rs"]
mod store_tests;
