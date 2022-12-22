use std::{
    cmp::{Ordering, PartialEq},
    collections::{BTreeMap, HashMap},
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
    _freq: BTreeMap<usize, _Lru<usize, Arc<String>>>, // shrinkable
    // _ttl: BinaryHeap<MinTTL>,
    _used_size: u64,
    _max_size: u64,
    _ttl_buf: u128,
}

impl Tlrfu {
    pub fn new(max_size: u64, ttl_buf: u128) -> Self {
        Self {
            _store: HashMap::new(),
            _freq: BTreeMap::new(),
            // _ttl: BinaryHeap::new(),
            _used_size: 0,
            _max_size: max_size,
            _ttl_buf: ttl_buf,
        }
    }

    fn _contains(&self, k: &String) -> bool {
        self._store.contains_key(k)
    }

    pub fn _is_size_exceeded(&self, bytes: u64) -> bool {
        self._used_size + bytes > self._max_size
    }

    pub fn _is_ttl_elapsed(&self) -> bool {
        false
    }

    pub async fn _get(&mut self, key: &String) -> Result<Option<&Vec<u8>>> {
        if let Some(data) = self._store.get_mut(key) {
            let lru = self._freq.get_mut(&data._freq).with_context(|| {
                format!("[TLRFU]: Key: {key} not found at freq {}.", data._freq)
            })?;
            let key = lru._remove(&data._key).await.with_context(|| {
                format!(
                    "[TLRFU]: Failed to remove LRU key: {} not found at freq {}.",
                    data._key, data._freq
                )
            })?;
            lru._is_empty().then(|| self._freq.remove(&data._freq));
            data._freq += 1;
            let lru = self._freq.entry(data._freq).or_insert(_Lru::_new(None));
            let lru_key = lru
                ._get_tail_key()
                .map(|tail_key| *tail_key + 1)
                .unwrap_or(0);
            lru._insert(lru_key, Arc::clone(&key))
                .await
                .with_context(|| {
                    format!("[LRU]: Failed to insert LRU with key: {lru_key}, value: {key}")
                })?;
            data._key = lru_key;
            Ok(Some(&data._value))
        } else {
            Ok(None)
        }
    }

    pub async fn _insert(&mut self, key: String, value: Vec<u8>) -> Result<()> {
        if self._contains(&key) {
            bail!("[TLRFU]: Key {key:?} existed while inserting.");
        }
        while self._is_size_exceeded(value.len() as u64) {
            let (&freq, lru) = self
                ._freq
                .iter_mut()
                .next()
                .context("[TLRFU]: Freq is empty while deleting. Maybe size too big?")?;
            let key = lru._remove_head().await?.with_context(|| {
                format!("[LRU]: Failed to get deleted head key at freq: {freq}")
            })?;
            let data = self
                ._store
                .remove(key.as_ref())
                .with_context(|| format!("[TLRFU]: Key {key} not found at store."))?;
            lru._is_empty().then(|| self._freq.remove(&freq));
            self._used_size -= data._value.len() as u64;
        }
        let key = Arc::new(key);
        let lru = self._freq.entry(1).or_insert(_Lru::_new(None));
        let lru_key = lru
            ._get_tail_key()
            .map(|tail_key| *tail_key + 1)
            .unwrap_or(0);
        lru._insert(lru_key, Arc::clone(&key))
            .await
            .with_context(|| {
                format!("[LRU]: Failed to insert LRU with key: {lru_key}, value: {key}")
            })?;
        self._used_size += value.len() as u64; // MAX = 2^64-1 bytes?
        self._store.insert(
            key,
            Data {
                _value: value,
                _freq: 1,
                _key: lru_key,
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
        let cache = Tlrfu::new(200_000_000, 0);
        assert_eq!(cache._store.len(), 0);
        assert_eq!(cache._freq.len(), 0);
        // assert_eq!(cache._ttl.len(), 0);
        assert_eq!(cache._used_size, 0);
        assert_eq!(cache._max_size, 200_000_000);
        assert_eq!(cache._ttl_buf, 0);
    }

    #[tokio::test]
    async fn insert_duplicate() {
        let mut cache = Tlrfu::new(200_000_000, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        assert!(cache._insert("a".into(), vec![0]).await.is_err());
    }

    #[tokio::test]
    async fn insert_one() {
        let mut cache = Tlrfu::new(200_000_000, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();

        assert_eq!(cache._store.len(), 1);

        let data = cache._store.get(&"a".to_string()).unwrap();
        assert_eq!(data._value, &[0]);
        assert_eq!(data._freq, 1);
        assert_eq!(data._key, 0);

        assert_eq!(cache._freq.len(), 1);

        let lru = cache._freq.get(&1).unwrap();
        assert_eq!(lru._len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"a".to_string());

        // assert_eq!(cache._ttl.len(), 1);
        assert_eq!(cache._used_size, 1);
    }

    #[tokio::test]
    async fn insert_two() {
        let mut cache = Tlrfu::new(200_000_000, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        cache._insert("b".into(), vec![1]).await.unwrap();

        assert_eq!(cache._store.len(), 2);

        let data = cache._store.get(&"b".to_string()).unwrap();
        assert_eq!(data._value, &[1]);
        assert_eq!(data._freq, 1);
        assert_eq!(data._key, 1);

        assert_eq!(cache._freq.len(), 1);

        let lru = cache._freq.get(&1).unwrap();
        assert_eq!(lru._len(), 2);
        assert_eq!(lru._get(&1).unwrap().as_ref(), &"b".to_string());

        // assert_eq!(cache._ttl.len(), 1);
        assert_eq!(cache._used_size, 2);
    }

    #[tokio::test]
    async fn get_empty() {
        let mut cache = Tlrfu::new(200_000_000, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        assert!(cache._get(&"b".to_string()).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn get_one_with_one_bucket() {
        let mut cache = Tlrfu::new(200_000_000, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        assert_eq!(cache._get(&"a".to_string()).await.unwrap().unwrap(), &[0]);

        let data = cache._store.get(&"a".to_string()).unwrap();
        assert_eq!(data._value, &[0]);
        assert_eq!(data._freq, 2);
        assert_eq!(data._key, 0);

        assert_eq!(cache._freq.len(), 1);

        let lru = cache._freq.get(&2).unwrap();
        assert_eq!(lru._len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"a".to_string());
    }

    #[tokio::test]
    async fn get_one_with_two_bucket() {
        let mut cache = Tlrfu::new(200_000_000, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        cache._insert("b".into(), vec![1]).await.unwrap();
        assert_eq!(cache._get(&"b".to_string()).await.unwrap().unwrap(), &[1]);

        let data = cache._store.get(&"b".to_string()).unwrap();
        assert_eq!(data._value, &[1]);
        assert_eq!(data._freq, 2);
        assert_eq!(data._key, 0);

        assert_eq!(cache._freq.len(), 2);

        let lru = cache._freq.get(&2).unwrap();
        assert_eq!(lru._len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"b".to_string());
    }

    #[tokio::test]
    async fn get_two_with_one_bucket() {
        let mut cache = Tlrfu::new(200_000_000, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        assert_eq!(cache._get(&"a".to_string()).await.unwrap().unwrap(), &[0]);
        assert_eq!(cache._get(&"a".to_string()).await.unwrap().unwrap(), &[0]);

        let data = cache._store.get(&"a".to_string()).unwrap();
        assert_eq!(data._value, &[0]);
        assert_eq!(data._freq, 3);
        assert_eq!(data._key, 0);

        assert_eq!(cache._freq.len(), 1);

        let lru = cache._freq.get(&3).unwrap();
        assert_eq!(lru._len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"a".to_string());
    }

    #[tokio::test]
    async fn get_two_with_two_bucket() {
        let mut cache = Tlrfu::new(200_000_000, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        cache._insert("b".into(), vec![1]).await.unwrap();
        assert_eq!(cache._get(&"b".to_string()).await.unwrap().unwrap(), &[1]);
        assert_eq!(cache._get(&"b".to_string()).await.unwrap().unwrap(), &[1]);

        let data = cache._store.get(&"b".to_string()).unwrap();
        assert_eq!(data._value, &[1]);
        assert_eq!(data._freq, 3);
        assert_eq!(data._key, 0);

        assert_eq!(cache._freq.len(), 2);

        let lru = cache._freq.get(&3).unwrap();
        assert_eq!(lru._len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"b".to_string());
    }

    #[tokio::test]
    async fn insert_exceed_cap_with_one_bucket() {
        let mut cache = Tlrfu::new(2, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        cache._insert("b".into(), vec![1]).await.unwrap();
        cache._insert("c".into(), vec![2]).await.unwrap();

        assert_eq!(cache._store.len(), 2);

        assert!(cache._store.get(&"a".to_string()).is_none());

        let data = cache._store.get(&"b".to_string()).unwrap();
        assert_eq!(data._value, &[1]);
        assert_eq!(data._freq, 1);
        assert_eq!(data._key, 1);

        let data = cache._store.get(&"c".to_string()).unwrap();
        assert_eq!(data._value, &[2]);
        assert_eq!(data._freq, 1);
        assert_eq!(data._key, 2);

        assert_eq!(cache._freq.len(), 1);

        let lru = cache._freq.get(&1).unwrap();
        assert_eq!(lru._len(), 2);
        assert!(lru._get(&0).is_none());
        assert_eq!(lru._get(&1).unwrap().as_ref(), &"b".to_string());
        assert_eq!(lru._get(&2).unwrap().as_ref(), &"c".to_string());

        // assert_eq!(cache._ttl.len(), 1);
        assert_eq!(cache._used_size, 2);
    }

    #[tokio::test]
    async fn insert_exceed_cap_with_two_bucket() {
        let mut cache = Tlrfu::new(2, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        cache._insert("b".into(), vec![1]).await.unwrap();

        assert_eq!(cache._get(&"a".to_string()).await.unwrap().unwrap(), &[0]);
        assert_eq!(cache._get(&"a".to_string()).await.unwrap().unwrap(), &[0]);
        assert_eq!(cache._get(&"b".to_string()).await.unwrap().unwrap(), &[1]);

        cache._insert("c".into(), vec![2]).await.unwrap();

        assert_eq!(cache._store.len(), 2);

        let data = cache._store.get(&"a".to_string()).unwrap();
        assert_eq!(data._value, &[0]);
        assert_eq!(data._freq, 3);
        assert_eq!(data._key, 0);

        assert!(cache._store.get(&"b".to_string()).is_none());

        let data = cache._store.get(&"c".to_string()).unwrap();
        assert_eq!(data._value, &[2]);
        assert_eq!(data._freq, 1);
        assert_eq!(data._key, 0);

        assert_eq!(cache._freq.len(), 2);

        let lru = cache._freq.get(&1).unwrap();
        assert_eq!(lru._len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"c".to_string());

        let lru = cache._freq.get(&3).unwrap();
        assert_eq!(lru._len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"a".to_string());

        // assert_eq!(cache._ttl.len(), 1);
        assert_eq!(cache._used_size, 2);
    }

    #[tokio::test]
    async fn insert_exceed_cap_with_many_buckets_deleted() {
        let mut cache = Tlrfu::new(3, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        cache._insert("b".into(), vec![1]).await.unwrap();
        cache._insert("c".into(), vec![2]).await.unwrap();
        cache._insert("d".into(), vec![3, 4, 5]).await.unwrap();

        assert_eq!(cache._store.len(), 1);

        assert!(cache._store.get(&"a".to_string()).is_none());
        assert!(cache._store.get(&"b".to_string()).is_none());
        assert!(cache._store.get(&"c".to_string()).is_none());
        let data = cache._store.get(&"d".to_string()).unwrap();
        assert_eq!(data._value, &[3, 4, 5]);
        assert_eq!(data._freq, 1);
        assert_eq!(data._key, 0);

        assert_eq!(cache._freq.len(), 1);

        let lru = cache._freq.get(&1).unwrap();
        assert_eq!(lru._len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"d".to_string());

        // assert_eq!(cache._ttl.len(), 1);
        assert_eq!(cache._used_size, 3);
    }
}
