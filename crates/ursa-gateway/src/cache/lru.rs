use {
    anyhow::{bail, Context, Result},
    std::{collections::HashMap, fmt::Debug, hash::Hash, sync::Arc},
    tokio::sync::RwLock,
};

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

#[cfg(test)]
mod tests {
    use super::*;

    impl<K, V> Lru<K, V>
    where
        K: Hash + Eq,
    {
        async fn ref_from_head(&self) -> Vec<Arc<K>> {
            let mut items = vec![];
            let mut head = Arc::clone(&self.head);
            'walk: loop {
                head = if let Some(node) = head.as_ref() {
                    items.push(Arc::clone(&node.data));
                    Arc::clone(&*node.next.read().await)
                } else {
                    break 'walk;
                };
            }
            items
        }
        async fn ref_from_tail(&self) -> Vec<Arc<K>> {
            let mut items = vec![];
            let mut tail = Arc::clone(&self.tail);
            'walk: loop {
                tail = if let Some(node) = tail.as_ref() {
                    items.push(Arc::clone(&node.data));
                    Arc::clone(&*node.prev.read().await)
                } else {
                    break 'walk;
                }
            }
            items
        }
    }

    fn ref_to_key<'a, K: 'a>(vec: &'a [Arc<K>]) -> Vec<&'a K> {
        vec.iter().map(|k| k.as_ref()).collect()
    }

    mod no_cap {
        use super::*;

        #[tokio::test]
        async fn new() {
            let lru = Lru::<&str, u8>::new(None);
            assert_eq!(lru.cap, None);
            assert_eq!(lru._len(), 0);
            assert!(lru.ref_from_head().await.is_empty());
            assert!(lru.ref_from_tail().await.is_empty());
        }

        #[tokio::test]
        async fn get_empty() {
            let lru = Lru::<&str, u8>::new(None);
            let res = lru._get(&"a");
            assert_eq!(res, None);
        }

        #[tokio::test]
        async fn remove_empty() {
            let mut lru = Lru::<&str, u8>::new(None);
            let res = lru.remove(&"a").await;
            assert_eq!(res, None);
        }

        #[tokio::test]
        async fn get_one() {
            let mut lru = Lru::new(None);
            lru.insert("a", 1).await.unwrap();
            assert_eq!(lru._get(&"a"), Some(&1));
        }

        #[tokio::test]
        async fn get_two() {
            let mut lru = Lru::new(None);
            lru.insert("a", 1).await.unwrap();
            lru.insert("b", 2).await.unwrap();
            assert_eq!(lru._get(&"a"), Some(&1));
            assert_eq!(lru._get(&"b"), Some(&2));
        }

        #[tokio::test]
        async fn get_three() {
            let mut lru = Lru::new(None);
            lru.insert("a", 1).await.unwrap();
            lru.insert("b", 2).await.unwrap();
            lru.insert("c", 3).await.unwrap();
            assert_eq!(lru._get(&"a"), Some(&1));
            assert_eq!(lru._get(&"b"), Some(&2));
            assert_eq!(lru._get(&"c"), Some(&3));
        }

        #[tokio::test]
        async fn remove_one() {
            let mut lru = Lru::new(None);
            lru.insert("a", 1).await.unwrap();
            lru.remove(&"a").await;
            assert_eq!(lru._len(), 0);
            assert!(lru.ref_from_head().await.is_empty());
            assert!(lru.ref_from_tail().await.is_empty());
        }

        #[tokio::test]
        async fn remove_head() {
            let mut lru = Lru::new(None);
            lru.insert("a", 1).await.unwrap();
            lru.insert("b", 2).await.unwrap();
            lru.remove_head().await.unwrap();
            assert_eq!(lru._len(), 1);
            assert_eq!(lru._get(&"a"), None);
            assert_eq!(lru._get(&"b"), Some(&2));
            assert_eq!(ref_to_key(&lru.ref_from_head().await), [&"b"]);
            assert_eq!(ref_to_key(&lru.ref_from_tail().await), [&"b"]);
        }

        #[tokio::test]
        async fn remove_tail() {
            let mut lru = Lru::new(None);
            lru.insert("a", 1).await.unwrap();
            lru.insert("b", 2).await.unwrap();
            lru.remove(&"b").await;
            assert_eq!(lru._len(), 1);
            assert_eq!(lru._get(&"a"), Some(&1));
            assert_eq!(lru._get(&"b"), None);
            assert_eq!(ref_to_key(&lru.ref_from_head().await), [&"a"]);
            assert_eq!(ref_to_key(&lru.ref_from_tail().await), [&"a"]);
        }

        #[tokio::test]
        async fn remove_mid() {
            let mut lru = Lru::new(None);
            lru.insert("a", 1).await.unwrap();
            lru.insert("b", 2).await.unwrap();
            lru.insert("c", 3).await.unwrap();
            lru.remove(&"b").await;
            assert_eq!(lru._len(), 2);
            assert_eq!(lru._get(&"a"), Some(&1));
            assert_eq!(lru._get(&"b"), None);
            assert_eq!(lru._get(&"c"), Some(&3));
            assert_eq!(ref_to_key(&lru.ref_from_head().await), [&"a", &"c"]);
            assert_eq!(ref_to_key(&lru.ref_from_tail().await), [&"c", &"a"]);
        }

        #[tokio::test]
        async fn insert_duplicate() {
            let mut lru = Lru::new(None);
            lru.insert("a", 1).await.unwrap();
            assert!(lru.insert("a", 1).await.is_err());
            assert_eq!(lru._len(), 1);
            assert_eq!(ref_to_key(&lru.ref_from_head().await), [&"a"]);
            assert_eq!(ref_to_key(&lru.ref_from_tail().await), [&"a"]);
        }

        #[tokio::test]
        async fn insert_one() {
            let mut lru = Lru::new(None);
            lru.insert("a", 1).await.unwrap();
            assert_eq!(lru._len(), 1);
            assert_eq!(ref_to_key(&lru.ref_from_head().await), [&"a"]);
            assert_eq!(ref_to_key(&lru.ref_from_tail().await), [&"a"]);
        }

        #[tokio::test]
        async fn insert_two() {
            let mut lru = Lru::new(None);
            lru.insert("a", 1).await.unwrap();
            lru.insert("b", 2).await.unwrap();
            assert_eq!(lru._len(), 2);
            assert_eq!(ref_to_key(&lru.ref_from_head().await), [&"a", &"b"]);
            assert_eq!(ref_to_key(&lru.ref_from_tail().await), [&"b", &"a"]);
        }

        #[tokio::test]
        async fn insert_three() {
            let mut lru = Lru::new(None);
            lru.insert("a", 1).await.unwrap();
            lru.insert("b", 2).await.unwrap();
            lru.insert("c", 3).await.unwrap();
            assert_eq!(lru._len(), 3);
            assert_eq!(ref_to_key(&lru.ref_from_head().await), [&"a", &"b", &"c"]);
            assert_eq!(ref_to_key(&lru.ref_from_tail().await), [&"c", &"b", &"a"]);
        }
    }

    mod cap {
        use super::*;

        #[tokio::test]
        async fn new() {
            let lru = Lru::<&str, u8>::new(Some(3));
            assert_eq!(lru.cap, Some(3));
            assert!(lru.ref_from_head().await.is_empty());
            assert!(lru.ref_from_tail().await.is_empty());
        }

        #[tokio::test]
        async fn insert_exceed_with_cap_0() {
            let mut lru = Lru::new(Some(0));
            assert!(lru.insert("a", 1).await.is_err());
        }

        #[tokio::test]
        async fn insert_exceed_with_cap_1() {
            let mut lru = Lru::new(Some(1));
            lru.insert("a", 1).await.unwrap();
            lru.insert("b", 2).await.unwrap();
            lru.insert("c", 3).await.unwrap();
            lru.insert("d", 4).await.unwrap();
            assert_eq!(lru._len(), 1);
            assert_eq!(ref_to_key(&lru.ref_from_head().await), [&"d"]);
            assert_eq!(ref_to_key(&lru.ref_from_tail().await), [&"d"]);
            assert_eq!(lru._get(&"a"), None);
            assert_eq!(lru._get(&"b"), None);
            assert_eq!(lru._get(&"c"), None);
            assert_eq!(lru._get(&"d"), Some(&4));
        }

        #[tokio::test]
        async fn insert_exceed_with_cap_3() {
            let mut lru = Lru::new(Some(3));
            lru.insert("a", 1).await.unwrap();
            lru.insert("b", 2).await.unwrap();
            lru.insert("c", 3).await.unwrap();
            lru.insert("d", 4).await.unwrap();
            assert_eq!(lru._len(), 3);
            assert_eq!(ref_to_key(&lru.ref_from_head().await), [&"b", &"c", &"d"]);
            assert_eq!(ref_to_key(&lru.ref_from_tail().await), [&"d", &"c", &"b"]);
            assert_eq!(lru._get(&"a"), None);
            assert_eq!(lru._get(&"b"), Some(&2));
            assert_eq!(lru._get(&"c"), Some(&3));
            assert_eq!(lru._get(&"d"), Some(&4));
        }
    }
}
