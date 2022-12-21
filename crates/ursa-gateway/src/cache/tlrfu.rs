use std::{
    cmp::{Ordering, PartialEq},
    collections::HashMap,
    sync::Arc, // time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};

use super::lru::_Lru;

#[derive(PartialEq, Eq)]
struct MinTTL {
    key: String,
    ttl: u128,
}
impl PartialOrd for MinTTL {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.ttl.partial_cmp(&self.ttl)
    }
}
impl Ord for MinTTL {
    fn cmp(&self, other: &Self) -> Ordering {
        other.ttl.cmp(&self.ttl)
    }
}

struct Data {
    _value: Vec<u8>,
    _freq: usize,
    _key: usize,
}

pub struct Tlrfu {
    _store: HashMap<Arc<String>, Data>,
    _freq: HashMap<usize, _Lru<usize, Arc<String>>>,
    // _size: HashMap<String, u64>,
    // _ttl: BinaryHeap<MinTTL>,
    _used_size: u64,
    _max_size: u64,
    _ttl_buf: u128,
}

impl Tlrfu {
    pub fn new(max_size: u64, ttl_buf: u128) -> Self {
        Self {
            _store: HashMap::new(),
            _freq: HashMap::new(),
            // _size: HashMap::new(),
            // _ttl: BinaryHeap::new(),
            _used_size: 0,
            _max_size: max_size,
            _ttl_buf: ttl_buf,
        }
    }

    fn _contains(&self, k: &String) -> bool {
        self._store.contains_key(k)
    }

    pub fn _is_size_exceeded(&self) -> bool {
        self._used_size >= self._max_size
    }

    pub fn _is_ttl_elapsed(&self) -> bool {
        false
    }

    pub async fn _get(&mut self, key: &String) -> Result<Option<&Vec<u8>>> {
        if let Some(data) = self._store.get_mut(key) {
            let key = if let Some(lru) = self._freq.get_mut(&data._freq) {
                let key = lru._remove(&data._key).await.with_context(|| {
                    format!("[LRU]: Key: {} not found at freq {}", data._key, data._freq)
                })?;
                if lru.len() == 0 {
                    self._freq.remove(&data._freq);
                }
                key
            } else {
                bail!("[TLRFU]: Key: {key} not found at freq {}.", data._freq);
            };

            let lru = self._freq.entry(data._freq + 1).or_insert(_Lru::_new(None));
            lru._insert(lru.len(), key).await?;
            data._freq += 1;
            data._key = lru.len() - 1;

            Ok(Some(&data._value))
        } else {
            Ok(None)
        }
    }

    pub async fn _insert(&mut self, key: String, value: Vec<u8>) -> Result<()> {
        if self._contains(&key) {
            bail!("[TLRFU]: Key {key:?} existed");
        }
        let key = Arc::new(key);
        let lru = self._freq.entry(0).or_insert(_Lru::_new(None));
        lru._insert(lru.len(), Arc::clone(&key)).await?;
        self._used_size += value.len() as u64; // MAX=2^64-1?
        self._store.insert(
            key,
            Data {
                _value: value,
                _freq: 0,
                _key: lru.len() - 1,
            },
        );
        Ok(())
    }

    /*
     * fn _process(&mut self, key: &String) {
     * }
     */

    pub fn purge(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn new() {
        let cache = Tlrfu::new(0, 0);
        assert_eq!(cache._store.len(), 0);
        assert_eq!(cache._freq.len(), 0);
        // assert_eq!(cache._size.len(), 0);
        // assert_eq!(cache._ttl.len(), 0);
        assert_eq!(cache._used_size, 0);
        assert_eq!(cache._max_size, 0);
        assert_eq!(cache._ttl_buf, 0);
    }

    #[tokio::test]
    async fn insert_one() {
        let mut cache = Tlrfu::new(0, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();

        assert_eq!(cache._store.len(), 1);

        let data = cache._store.get(&"a".to_string()).unwrap();
        assert_eq!(data._value, &[0]);
        assert_eq!(data._freq, 0);
        assert_eq!(data._key, 0);

        assert_eq!(cache._freq.len(), 1);

        let lru = cache._freq.get(&0).unwrap();
        assert_eq!(lru.len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"a".to_string());

        // assert_eq!(cache._size.len(), 1);
        // assert_eq!(cache._ttl.len(), 1);
        assert_eq!(cache._used_size, 1);
    }

    #[tokio::test]
    async fn insert_two() {
        let mut cache = Tlrfu::new(0, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        cache._insert("b".into(), vec![1]).await.unwrap();

        assert_eq!(cache._store.len(), 2);

        let data = cache._store.get(&"b".to_string()).unwrap();
        assert_eq!(data._value, &[1]);
        assert_eq!(data._freq, 0);
        assert_eq!(data._key, 1);

        assert_eq!(cache._freq.len(), 1);

        let lru = cache._freq.get(&0).unwrap();
        assert_eq!(lru.len(), 2);
        assert_eq!(lru._get(&1).unwrap().as_ref(), &"b".to_string());

        // assert_eq!(cache._size.len(), 1);
        // assert_eq!(cache._ttl.len(), 1);
        assert_eq!(cache._used_size, 2);
    }

    #[tokio::test]
    async fn get_one_with_one_bucket() {
        let mut cache = Tlrfu::new(0, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        assert_eq!(cache._get(&"a".to_string()).await.unwrap().unwrap(), &[0]);

        let data = cache._store.get(&"a".to_string()).unwrap();
        assert_eq!(data._value, &[0]);
        assert_eq!(data._freq, 1);
        assert_eq!(data._key, 0);

        assert_eq!(cache._freq.len(), 1);

        let lru = cache._freq.get(&1).unwrap();
        assert_eq!(lru.len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"a".to_string());
    }

    #[tokio::test]
    async fn get_one_with_two_bucket() {
        let mut cache = Tlrfu::new(0, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        cache._insert("b".into(), vec![1]).await.unwrap();
        assert_eq!(cache._get(&"b".to_string()).await.unwrap().unwrap(), &[1]);

        let data = cache._store.get(&"b".to_string()).unwrap();
        assert_eq!(data._value, &[1]);
        assert_eq!(data._freq, 1);
        assert_eq!(data._key, 0);

        assert_eq!(cache._freq.len(), 2);

        let lru = cache._freq.get(&1).unwrap();
        assert_eq!(lru.len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"b".to_string());
    }

    #[tokio::test]
    async fn get_two_with_one_bucket() {
        let mut cache = Tlrfu::new(0, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        assert_eq!(cache._get(&"a".to_string()).await.unwrap().unwrap(), &[0]);
        assert_eq!(cache._get(&"a".to_string()).await.unwrap().unwrap(), &[0]);

        let data = cache._store.get(&"a".to_string()).unwrap();
        assert_eq!(data._value, &[0]);
        assert_eq!(data._freq, 2);
        assert_eq!(data._key, 0);

        assert_eq!(cache._freq.len(), 1);

        let lru = cache._freq.get(&2).unwrap();
        assert_eq!(lru.len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"a".to_string());
    }

    #[tokio::test]
    async fn get_two_with_two_bucket() {
        let mut cache = Tlrfu::new(0, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        cache._insert("b".into(), vec![1]).await.unwrap();
        assert_eq!(cache._get(&"b".to_string()).await.unwrap().unwrap(), &[1]);
        assert_eq!(cache._get(&"b".to_string()).await.unwrap().unwrap(), &[1]);

        let data = cache._store.get(&"b".to_string()).unwrap();
        assert_eq!(data._value, &[1]);
        assert_eq!(data._freq, 2);
        assert_eq!(data._key, 0);

        assert_eq!(cache._freq.len(), 2);

        let lru = cache._freq.get(&2).unwrap();
        assert_eq!(lru.len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"b".to_string());
    }
}
