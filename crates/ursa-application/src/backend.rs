use crate::interface::application::{
    Backend, ExecutionError, ProofOfConsensus, ProofOfMisbehavior, PublicKey, TableRef, Transaction,
};
use atomo::mt::{TableRef as AtomoTableRef, TableSelector};
use atomo::SerdeBackend;
use serde::de::DeserializeOwned;
use serde::Serialize;

use std::{any::Any, cell::RefCell, hash::Hash};

pub struct AtomoBackend<'selector, S: SerdeBackend> {
    pub table_selector: &'selector TableSelector<S>,
}

impl<'selector, S: SerdeBackend> Backend for AtomoBackend<'selector, S> {
    type Ref<
        K: Hash + Eq + Serialize + DeserializeOwned + Send + 'static,
        V: Serialize + DeserializeOwned + Clone + Send + 'static,
    > = AtomoTable<'selector, K, V, S>;

    fn get_table_reference<
        K: Hash + Eq + Serialize + DeserializeOwned + Send,
        V: Serialize + DeserializeOwned + Send + Clone,
    >(
        &self,
        id: &str,
    ) -> Self::Ref<K, V> {
        AtomoTable(RefCell::new(self.table_selector.get_table(id)))
    }

    fn verify_transaction(&self, txn: &Transaction) -> anyhow::Result<(), ExecutionError> {
        Ok(())
    }

    fn verify_proof_of_delivery(
        &self,
        client: &PublicKey,
        provider: &PublicKey,
        commodity: &u128,
        service_id: &u64,
        proof: (),
    ) -> bool {
        true
    }

    fn verify_proof_of_consensus(&self, proof: ProofOfConsensus) -> bool {
        true
    }

    fn verify_proof_of_misbehavior(&self, proof: ProofOfMisbehavior) -> bool {
        true
    }
}

pub struct AtomoTable<
    'selector,
    K: Hash + Eq + Serialize + DeserializeOwned + 'static,
    V: Serialize + DeserializeOwned + 'static,
    S: SerdeBackend,
>(RefCell<AtomoTableRef<'selector, K, V, S>>);

impl<
        'selector,
        K: Hash + Eq + Serialize + DeserializeOwned + Any,
        V: Serialize + DeserializeOwned + Any + Clone,
        S: SerdeBackend,
    > TableRef<K, V> for AtomoTable<'selector, K, V, S>
{
    fn set(&self, key: K, value: V) {
        self.0.borrow_mut().insert(key, value);
    }

    fn get(&self, key: &K) -> Option<V> {
        match self.0.borrow_mut().get(key) {
            Some(x) => Some(x.into_inner()),
            None => None,
        }
    }
}
