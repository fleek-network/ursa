use std::{
    cmp::{Ordering, PartialEq},
    collections::{BinaryHeap, HashMap},
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

pub struct TLRFUCache {
    _store: HashMap<Arc<String>, Data>,
    _freq: HashMap<usize, _Lru<usize, Arc<String>>>,
    _size: HashMap<String, u64>,
    _ttl: BinaryHeap<MinTTL>,
    _used_size: u64,
    _max_size: u64,
    _ttl_buf: u128,
}

impl TLRFUCache {
    pub fn new(max_size: u64, ttl_buf: u128) -> Self {
        Self {
            _store: HashMap::new(),
            _freq: HashMap::new(),
            _size: HashMap::new(),
            _ttl: BinaryHeap::new(),
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
            let _key = if let Some(lru) = self._freq.get_mut(&data._freq) {
                lru._remove(&data._key).await.with_context(|| {
                    format!("[LRU]: Key: {} not found at freq {}", data._key, data._freq)
                })?
            } else {
                bail!("[TLRFUCache]: Key: {key} not found at freq {}.", data._freq);
            };

            data._freq += 1;
            let lru = self._freq.entry(data._freq).or_insert(_Lru::_new(None));
            lru._insert(lru.len(), _key).await?;

            Ok(Some(&data._value))
        } else {
            Ok(None)
        }
    }

    pub async fn _insert(&mut self, key: String, value: Vec<u8>) -> Result<()> {
        if self._contains(&key) {
            return Ok(());
        }
        let key = Arc::new(key);
        let lru = self._freq.entry(0).or_insert(_Lru::_new(None));
        lru._insert(0, Arc::clone(&key)).await?;
        self._store.insert(
            key,
            Data {
                _value: value,
                _freq: 0,
                _key: 0,
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
