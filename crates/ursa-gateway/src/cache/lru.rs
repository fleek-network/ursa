use std::{collections::HashMap, fmt::Debug, hash::Hash, sync::Arc};

use anyhow::{bail, Context, Result};
use tokio::sync::RwLock;

struct _Node<T> {
    _next: RwLock<Arc<Option<_Node<T>>>>,
    _prev: RwLock<Arc<Option<_Node<T>>>>,
    _data: Arc<T>,
}

struct _Data<K, V> {
    _value: V,
    _node: Arc<Option<_Node<K>>>,
}

pub struct _Lru<K, V> {
    _store: HashMap<Arc<K>, _Data<K, V>>,
    _head: Arc<Option<_Node<K>>>,
    _tail: Arc<Option<_Node<K>>>,
    _cap: Option<usize>,
}

impl<K, V> _Lru<K, V>
where
    K: Hash + Eq + Debug,
{
    pub fn _new(cap: Option<usize>) -> Self {
        let nil = Arc::new(None);
        Self {
            _store: cap.map(HashMap::with_capacity).unwrap_or(HashMap::new()),
            _head: Arc::clone(&nil),
            _tail: nil,
            _cap: cap,
        }
    }

    fn _get_first_key(&self) -> Option<Arc<K>> {
        self._head
            .as_ref()
            .as_ref()
            .map(|node| Arc::clone(&node._data))
    }

    pub fn _get_tail_key(&self) -> Option<&K> {
        self._tail.as_ref().as_ref().map(|node| node._data.as_ref())
    }

    fn _contains(&self, k: &K) -> bool {
        self._store.contains_key(k)
    }

    pub fn _is_empty(&self) -> bool {
        self._store.is_empty()
    }

    pub fn _len(&self) -> usize {
        self._store.len()
    }

    pub fn _get(&self, k: &K) -> Option<&V> {
        self._store.get(k).map(|data| &data._value)
    }

    pub async fn _insert(&mut self, k: K, v: V) -> Result<()> {
        if self._contains(&k) {
            bail!("[LRU]: Key {k:?} existed");
        }
        if let Some(cap) = self._cap {
            if cap <= self._store.len() {
                self._remove_head().await?;
            }
        }
        let key = Arc::new(k);
        let new_tail = Arc::new(Some(_Node {
            _next: RwLock::new(Arc::new(None)),
            _prev: RwLock::new(Arc::clone(&self._tail)),
            _data: Arc::clone(&key),
        }));
        if let Some(old_tail) = self._tail.as_ref() {
            *old_tail._next.write().await = Arc::clone(&new_tail);
        }
        self._store.insert(
            key,
            _Data {
                _value: v,
                _node: Arc::clone(&new_tail),
            },
        );
        self._tail = Arc::clone(&new_tail);
        self._head.as_ref().is_none().then(|| self._head = new_tail);
        Ok(())
    }

    pub async fn _remove_head(&mut self) -> Result<Option<V>> {
        let first_key = self
            ._get_first_key()
            .context("[LRU]: Failed to get the first key while deleting.")?;
        Ok(self._remove(first_key.as_ref()).await)
    }

    pub async fn _remove(&mut self, k: &K) -> Option<V> {
        if let Some(data) = self._store.remove(k) {
            if let Some(node) = data._node.as_ref() {
                let prev = node._prev.read().await;
                let next = node._next.read().await;
                if let Some(next) = next.as_ref() {
                    *next._prev.write().await = Arc::clone(&prev);
                } else {
                    self._tail = Arc::clone(&prev);
                }
                if let Some(prev) = prev.as_ref() {
                    *prev._next.write().await = Arc::clone(&next);
                } else {
                    self._head = Arc::clone(&next);
                }
            }
            Some(data._value)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl<K, V> _Lru<K, V>
    where
        K: Hash + Eq,
    {
        pub async fn ref_from_head(&self) -> Vec<Arc<K>> {
            let mut items = vec![];
            let mut head = Arc::clone(&self._head);
            'walk: loop {
                head = if let Some(node) = head.as_ref() {
                    items.push(Arc::clone(&node._data));
                    Arc::clone(&*node._next.read().await)
                } else {
                    break 'walk;
                };
            }
            items
        }
        pub async fn ref_from_tail(&self) -> Vec<Arc<K>> {
            let mut items = vec![];
            let mut tail = Arc::clone(&self._tail);
            'walk: loop {
                tail = if let Some(node) = tail.as_ref() {
                    items.push(Arc::clone(&node._data));
                    Arc::clone(&*node._prev.read().await)
                } else {
                    break 'walk;
                }
            }
            items
        }
    }

    pub fn ref_to_key<'a, K: 'a>(vec: &'a [Arc<K>]) -> Vec<&'a K> {
        vec.iter().map(|k| k.as_ref()).collect()
    }

    mod no_cap {
        use super::*;

        #[tokio::test]
        async fn new() {
            let lru = _Lru::<&str, u8>::_new(None);
            assert_eq!(lru._cap, None);
            assert_eq!(lru._len(), 0);
            assert!(lru.ref_from_head().await.is_empty());
            assert!(lru.ref_from_tail().await.is_empty());
        }

        #[tokio::test]
        async fn get_empty() {
            let lru = _Lru::<&str, u8>::_new(None);
            let res = lru._get(&"a");
            assert_eq!(res, None);
        }

        #[tokio::test]
        async fn remove_empty() {
            let mut lru = _Lru::<&str, u8>::_new(None);
            let res = lru._remove(&"a").await;
            assert_eq!(res, None);
        }

        #[tokio::test]
        async fn get_one() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await.unwrap();
            assert_eq!(lru._get(&"a"), Some(&1));
        }

        #[tokio::test]
        async fn get_two() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await.unwrap();
            lru._insert("b", 2).await.unwrap();
            assert_eq!(lru._get(&"a"), Some(&1));
            assert_eq!(lru._get(&"b"), Some(&2));
        }

        #[tokio::test]
        async fn get_three() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await.unwrap();
            lru._insert("b", 2).await.unwrap();
            lru._insert("c", 3).await.unwrap();
            assert_eq!(lru._get(&"a"), Some(&1));
            assert_eq!(lru._get(&"b"), Some(&2));
            assert_eq!(lru._get(&"c"), Some(&3));
        }

        #[tokio::test]
        async fn remove_one() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await.unwrap();
            lru._remove(&"a").await;
            assert_eq!(lru._len(), 0);
            assert!(lru.ref_from_head().await.is_empty());
            assert!(lru.ref_from_tail().await.is_empty());
        }

        #[tokio::test]
        async fn remove_head() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await.unwrap();
            lru._insert("b", 2).await.unwrap();
            lru._remove_head().await.unwrap();
            assert_eq!(lru._len(), 1);
            assert_eq!(lru._get(&"a"), None);
            assert_eq!(lru._get(&"b"), Some(&2));
            assert_eq!(ref_to_key(&lru.ref_from_head().await), [&"b"]);
            assert_eq!(ref_to_key(&lru.ref_from_tail().await), [&"b"]);
        }

        #[tokio::test]
        async fn remove_tail() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await.unwrap();
            lru._insert("b", 2).await.unwrap();
            lru._remove(&"b").await;
            assert_eq!(lru._len(), 1);
            assert_eq!(lru._get(&"a"), Some(&1));
            assert_eq!(lru._get(&"b"), None);
            assert_eq!(ref_to_key(&lru.ref_from_head().await), [&"a"]);
            assert_eq!(ref_to_key(&lru.ref_from_tail().await), [&"a"]);
        }

        #[tokio::test]
        async fn remove_mid() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await.unwrap();
            lru._insert("b", 2).await.unwrap();
            lru._insert("c", 3).await.unwrap();
            lru._remove(&"b").await;
            assert_eq!(lru._len(), 2);
            assert_eq!(lru._get(&"a"), Some(&1));
            assert_eq!(lru._get(&"b"), None);
            assert_eq!(lru._get(&"c"), Some(&3));
            assert_eq!(ref_to_key(&lru.ref_from_head().await), [&"a", &"c"]);
            assert_eq!(ref_to_key(&lru.ref_from_tail().await), [&"c", &"a"]);
        }

        #[tokio::test]
        async fn insert_duplicate() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await.unwrap();
            assert!(lru._insert("a", 1).await.is_err());
            assert_eq!(lru._len(), 1);
            assert_eq!(ref_to_key(&lru.ref_from_head().await), [&"a"]);
            assert_eq!(ref_to_key(&lru.ref_from_tail().await), [&"a"]);
        }

        #[tokio::test]
        async fn insert_one() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await.unwrap();
            assert_eq!(lru._len(), 1);
            assert_eq!(ref_to_key(&lru.ref_from_head().await), [&"a"]);
            assert_eq!(ref_to_key(&lru.ref_from_tail().await), [&"a"]);
        }

        #[tokio::test]
        async fn insert_two() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await.unwrap();
            lru._insert("b", 2).await.unwrap();
            assert_eq!(lru._len(), 2);
            assert_eq!(ref_to_key(&lru.ref_from_head().await), [&"a", &"b"]);
            assert_eq!(ref_to_key(&lru.ref_from_tail().await), [&"b", &"a"]);
        }

        #[tokio::test]
        async fn insert_three() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await.unwrap();
            lru._insert("b", 2).await.unwrap();
            lru._insert("c", 3).await.unwrap();
            assert_eq!(lru._len(), 3);
            assert_eq!(ref_to_key(&lru.ref_from_head().await), [&"a", &"b", &"c"]);
            assert_eq!(ref_to_key(&lru.ref_from_tail().await), [&"c", &"b", &"a"]);
        }
    }

    mod cap {
        use super::*;

        #[tokio::test]
        async fn new() {
            let lru = _Lru::<&str, u8>::_new(Some(3));
            assert_eq!(lru._cap, Some(3));
            assert!(lru.ref_from_head().await.is_empty());
            assert!(lru.ref_from_tail().await.is_empty());
        }

        #[tokio::test]
        async fn insert_exceed_with_cap_0() {
            let mut lru = _Lru::_new(Some(0));
            assert!(lru._insert("a", 1).await.is_err());
        }

        #[tokio::test]
        async fn insert_exceed_with_cap_1() {
            let mut lru = _Lru::_new(Some(1));
            lru._insert("a", 1).await.unwrap();
            lru._insert("b", 2).await.unwrap();
            lru._insert("c", 3).await.unwrap();
            lru._insert("d", 4).await.unwrap();
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
            let mut lru = _Lru::_new(Some(3));
            lru._insert("a", 1).await.unwrap();
            lru._insert("b", 2).await.unwrap();
            lru._insert("c", 3).await.unwrap();
            lru._insert("d", 4).await.unwrap();
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
