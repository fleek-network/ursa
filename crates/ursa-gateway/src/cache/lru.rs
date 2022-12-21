use std::{collections::HashMap, hash::Hash, sync::Arc};

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
    K: Hash + Eq,
{
    pub fn _new(cap: Option<usize>) -> Self {
        let nil = Arc::new(None);
        Self {
            _store: if let Some(cap) = cap {
                HashMap::with_capacity(cap)
            } else {
                HashMap::new()
            },
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

    fn _contains(&self, k: &K) -> bool {
        self._store.contains_key(k)
    }

    pub fn _get(&self, k: &K) -> Option<&V> {
        self._store.get(k).map(|data| &data._value)
    }

    pub async fn _insert(&mut self, k: K, v: V) {
        if self._contains(&k) {
            return;
        }
        if let Some(cap) = self._cap {
            if cap <= self._store.len() {
                let first_key = self
                    ._get_first_key()
                    .expect("[LRU]: Failed to get the first key while deleting.");
                self._remove(first_key.as_ref()).await;
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
        if self._head.as_ref().is_none() {
            self._head = new_tail;
        }
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
        pub async fn k_from_head(&self) -> Vec<Arc<K>> {
            let mut items = vec![];
            let mut head = Arc::clone(&self._head);
            'walk: loop {
                head = if let Some(node) = head.as_ref() {
                    items.push(Arc::clone(&node._data));
                    node._next.read().await.clone()
                } else {
                    break 'walk;
                };
            }
            items
        }
        pub async fn k_from_tail(&self) -> Vec<Arc<K>> {
            let mut items = vec![];
            let mut tail = Arc::clone(&self._tail);
            'walk: loop {
                tail = if let Some(node) = tail.as_ref() {
                    items.push(Arc::clone(&node._data));
                    node._prev.read().await.clone()
                } else {
                    break 'walk;
                }
            }
            items
        }
    }

    pub fn ref_to_k<K: Clone>(vec: Vec<Arc<K>>) -> Vec<K> {
        vec.into_iter().map(|k| k.as_ref().clone()).collect()
    }

    mod no_cap {
        use super::*;

        #[tokio::test]
        async fn new() {
            let lru = _Lru::<&str, u8>::_new(None);
            assert_eq!(lru._cap, None);
            assert!(lru.k_from_head().await.is_empty());
            assert!(lru.k_from_tail().await.is_empty());
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
            lru._insert("a", 1).await;
            assert_eq!(lru._get(&"a"), Some(&1));
        }

        #[tokio::test]
        async fn get_two() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await;
            lru._insert("b", 2).await;
            assert_eq!(lru._get(&"a"), Some(&1));
            assert_eq!(lru._get(&"b"), Some(&2));
        }

        #[tokio::test]
        async fn get_three() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await;
            lru._insert("b", 2).await;
            lru._insert("c", 3).await;
            assert_eq!(lru._get(&"a"), Some(&1));
            assert_eq!(lru._get(&"b"), Some(&2));
            assert_eq!(lru._get(&"c"), Some(&3));
        }

        #[tokio::test]
        async fn remove_one() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await;
            lru._remove(&"a").await;
            assert!(lru.k_from_head().await.is_empty());
            assert!(lru.k_from_tail().await.is_empty());
        }

        #[tokio::test]
        async fn remove_head() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await;
            lru._insert("b", 2).await;
            lru._remove(&"a").await;
            assert_eq!(lru._get(&"a"), None);
            assert_eq!(lru._get(&"b"), Some(&2));
            assert_eq!(ref_to_k(lru.k_from_head().await), ["b"]);
            assert_eq!(ref_to_k(lru.k_from_tail().await), ["b"]);
        }

        #[tokio::test]
        async fn remove_tail() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await;
            lru._insert("b", 2).await;
            lru._remove(&"b").await;
            assert_eq!(lru._get(&"a"), Some(&1));
            assert_eq!(lru._get(&"b"), None);
            assert_eq!(ref_to_k(lru.k_from_head().await), ["a"]);
            assert_eq!(ref_to_k(lru.k_from_tail().await), ["a"]);
        }

        #[tokio::test]
        async fn remove_mid() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await;
            lru._insert("b", 2).await;
            lru._insert("c", 3).await;
            lru._remove(&"b").await;
            assert_eq!(lru._get(&"a"), Some(&1));
            assert_eq!(lru._get(&"b"), None);
            assert_eq!(lru._get(&"c"), Some(&3));
            assert_eq!(ref_to_k(lru.k_from_head().await), ["a", "c"]);
            assert_eq!(ref_to_k(lru.k_from_tail().await), ["c", "a"]);
        }

        #[tokio::test]
        async fn insert_duplicate() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await;
            lru._insert("a", 1).await;
            assert_eq!(ref_to_k(lru.k_from_head().await), ["a"]);
            assert_eq!(ref_to_k(lru.k_from_tail().await), ["a"]);
        }

        #[tokio::test]
        async fn insert_one() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await;
            assert_eq!(ref_to_k(lru.k_from_head().await), ["a"]);
            assert_eq!(ref_to_k(lru.k_from_tail().await), ["a"]);
        }

        #[tokio::test]
        async fn insert_two() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await;
            lru._insert("b", 2).await;
            assert_eq!(ref_to_k(lru.k_from_head().await), ["a", "b"]);
            assert_eq!(ref_to_k(lru.k_from_tail().await), ["b", "a"]);
        }

        #[tokio::test]
        async fn insert_three() {
            let mut lru = _Lru::_new(None);
            lru._insert("a", 1).await;
            lru._insert("b", 2).await;
            lru._insert("c", 3).await;
            assert_eq!(ref_to_k(lru.k_from_head().await), ["a", "b", "c"]);
            assert_eq!(ref_to_k(lru.k_from_tail().await), ["c", "b", "a"]);
        }
    }

    mod cap {
        use super::*;

        #[tokio::test]
        async fn new() {
            let lru = _Lru::<&str, u8>::_new(Some(3));
            assert_eq!(lru._cap, Some(3));
            assert!(lru.k_from_head().await.is_empty());
            assert!(lru.k_from_tail().await.is_empty());
        }

        #[tokio::test]
        async fn insert_exceed_cap() {
            let mut lru = _Lru::_new(Some(3));
            lru._insert("a", 1).await;
            lru._insert("b", 2).await;
            lru._insert("c", 3).await;
            lru._insert("d", 4).await;
            assert_eq!(ref_to_k(lru.k_from_head().await), ["b", "c", "d"]);
            assert_eq!(ref_to_k(lru.k_from_tail().await), ["d", "c", "b"]);
            assert_eq!(lru._get(&"a"), None);
            assert_eq!(lru._get(&"b"), Some(&2));
            assert_eq!(lru._get(&"c"), Some(&3));
            assert_eq!(lru._get(&"d"), Some(&4));
        }
    }
}
