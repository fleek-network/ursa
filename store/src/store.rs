use ipld_blockstore::BlockStore;
use libipld::store::DefaultParams;
use libipld::{Block, Cid, Result};
use libp2p_bitswap::BitswapStore;
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

    pub fn blockstore(&self) -> &S {
        &self.db
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
        Ok(self.0.db.exists(cid.to_bytes())?)
    }

    fn get(&mut self, cid: &Cid) -> Result<Option<Vec<u8>>> {
        Ok(self.0.db.read(cid.to_bytes()).unwrap())
    }

    fn insert(&mut self, block: &Block<Self::Params>) -> Result<()> {
        self.0
            .db
            .write(&block.cid().to_bytes(), block.data())
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
