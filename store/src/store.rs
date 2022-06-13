use cid::Cid as Forest_Cid;
use ipld_blockstore::BlockStore;
use libipld::store::DefaultParams;
use libipld::{Block, Cid, Result};
use libp2p_bitswap::BitswapStore;
use std::str::FromStr;
use std::sync::Arc;

pub struct Store<S> {
    pub db: Arc<S>,
}

impl<S> Store<S>
where
    S: BlockStore + Send + Sync + 'static,
{
    pub fn new(db: Arc<S>) -> Self {
        Self { db }
    }
}
pub struct BitswapStorage<P>(pub Arc<Store<P>>)
where
    P: BlockStore + Sync + Send + 'static;

impl<P> BitswapStore for BitswapStorage<P>
where
    P: BlockStore + Sync + Send + 'static,
{
    type Params = DefaultParams;

    fn contains(&mut self, cid: &Cid) -> Result<bool> {
        let key = Forest_Cid::from_str(&cid.to_string());
        Ok(self.0.db.exists(key.unwrap().to_bytes())?)
    }

    fn get(&mut self, cid: &Cid) -> Result<Option<Vec<u8>>> {
        let key = Forest_Cid::from_str(&cid.to_string()).unwrap();
        Ok(self.0.db.read(key.to_bytes()).unwrap())
    }

    fn insert(&mut self, block: &Block<Self::Params>) -> Result<()> {
        let key = Forest_Cid::from_str(&block.cid().to_string()).unwrap();
        self.0
            .db
            .write(key.to_bytes(), block.data().to_vec())
            .unwrap();

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
