use anyhow::anyhow;
use async_trait::async_trait;
use cid::{
    multihash::{Code, MultihashDigest},
    Cid,
};
use db::Store;
use fnv::FnvHashSet;
use futures::{channel::mpsc::unbounded, SinkExt};
use fvm_ipld_blockstore::Blockstore;
use fvm_ipld_car::CarHeader;
use fvm_ipld_encoding::{de::DeserializeOwned, from_slice, ser::Serialize, to_vec, DAG_CBOR};
use ipld_traversal::blockstore::Blockstore as GSBlockstore;
use libipld::{store::DefaultParams, Block, Result};
use libp2p_bitswap::BitswapStore;
use std::sync::Arc;
use tokio::{sync::RwLock, task};

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

#[async_trait]
pub trait Dag {
    /// traverse a dag and get full dag given a root cid
    fn dag_traversal(&self, root_cid: &Cid) -> Result<Vec<(Cid, Vec<u8>)>>;

    /// Build a temporary car file from a root cid and return the size
    async fn car_size(&self, root_cid: &Cid) -> Result<u64>;
}

#[async_trait]
impl<S> Dag for UrsaStore<S>
where
    S: Blockstore + Sync + Send + 'static,
{
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

    /// Build a temporary car file from a root cid and return the size
    async fn car_size(&self, root_cid: &Cid) -> Result<u64> {
        let buf: Arc<RwLock<Vec<u8>>> = Default::default();
        let header = CarHeader {
            roots: vec![*root_cid],
            version: 1,
        };
        let (mut tx, mut rx) = unbounded();
        let buf_cloned = buf.clone();
        let write_task = task::spawn(async move {
            header
                .write_stream_async(&mut *buf_cloned.write().await, &mut rx)
                .await
                .unwrap()
        });
        let dag = self.dag_traversal(root_cid)?;
        for item in dag {
            tx.send(item).await.unwrap();
        }
        drop(tx);
        write_task.await?;

        let len = buf.read().await.len();
        Ok(len as u64)
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
