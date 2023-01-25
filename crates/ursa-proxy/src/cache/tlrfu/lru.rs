use std::{collections::HashMap, fmt::Debug, hash::Hash, sync::Arc};

use anyhow::{bail, Context, Result};
use tokio::sync::RwLock;

struct Node<T> {
    next: RwLock<Arc<Option<Node<T>>>>,
    prev: RwLock<Arc<Option<Node<T>>>>,
    data: Arc<T>,
}

struct Data<K, V> {
    value: V,
    node: Arc<Option<Node<K>>>,
}

pub struct Lru<K, V> {
    store: HashMap<Arc<K>, Data<K, V>>,
    head: Arc<Option<Node<K>>>,
    tail: Arc<Option<Node<K>>>,
    cap: Option<usize>,
}

impl<K, V> Lru<K, V>
where
    K: Hash + Eq + Debug,
{
    pub fn new(cap: Option<usize>) -> Self {
        let nil = Arc::new(None);
        Self {
            store: cap.map(HashMap::with_capacity).unwrap_or_default(),
            head: Arc::clone(&nil),
            tail: nil,
            cap,
        }
    }

    pub fn get_tail_key(&self) -> Option<&K> {
        self.tail.as_ref().as_ref().map(|node| node.data.as_ref())
    }

    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    pub fn _len(&self) -> usize {
        self.store.len()
    }

    pub fn _get(&self, k: &K) -> Option<&V> {
        self.store.get(k).map(|data| &data.value)
    }

    pub async fn insert(&mut self, k: K, v: V) -> Result<()> {
        if self.contains(&k) {
            bail!("[LRU]: Key {k:?} existed while inserting");
        }
        if let Some(cap) = self.cap {
            if cap <= self.store.len() {
                self.remove_head().await?;
            }
        }
        let key = Arc::new(k);
        let tail = Arc::new(Some(Node {
            next: RwLock::new(Arc::new(None)),
            prev: RwLock::new(Arc::clone(&self.tail)),
            data: Arc::clone(&key),
        }));
        if let Some(old_tail) = self.tail.as_ref() {
            *old_tail.next.write().await = Arc::clone(&tail);
        }
        self.store.insert(
            key,
            Data {
                value: v,
                node: Arc::clone(&tail),
            },
        );
        self.tail = Arc::clone(&tail);
        self.head.as_ref().is_none().then(|| self.head = tail);
        Ok(())
    }

    pub async fn remove_head(&mut self) -> Result<Option<V>> {
        let first_key = self
            .get_first_key()
            .context("[LRU]: Failed to get the first key while deleting")?;
        Ok(self.remove(first_key.as_ref()).await)
    }

    pub async fn remove(&mut self, k: &K) -> Option<V> {
        if let Some(data) = self.store.remove(k) {
            if let Some(node) = data.node.as_ref() {
                let prev = node.prev.read().await;
                let next = node.next.read().await;
                if let Some(next) = next.as_ref() {
                    *next.prev.write().await = Arc::clone(&prev);
                } else {
                    self.tail = Arc::clone(&prev);
                }
                if let Some(prev) = prev.as_ref() {
                    *prev.next.write().await = Arc::clone(&next);
                } else {
                    self.head = Arc::clone(&next);
                }
            }
            Some(data.value)
        } else {
            None
        }
    }

    fn get_first_key(&self) -> Option<Arc<K>> {
        self.head
            .as_ref()
            .as_ref()
            .map(|node| Arc::clone(&node.data))
    }

    fn contains(&self, k: &K) -> bool {
        self.store.contains_key(k)
    }
}
