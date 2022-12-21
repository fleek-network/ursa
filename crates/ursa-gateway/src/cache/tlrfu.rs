use std::{
    cmp::{Ordering, PartialEq},
    collections::{BinaryHeap, HashMap},
    // time::{SystemTime, UNIX_EPOCH},
};

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
    _key: String,
}

pub struct TLRFUCache {
    _store: HashMap<String, Data>,
    _freq: HashMap<usize, _Lru<usize, String>>,
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

    pub fn _is_size_exceeded(&self) -> bool {
        self._used_size >= self._max_size
    }

    pub fn _is_ttl_elapsed(&self) -> bool {
        false
    }

    pub fn _get(&self, key: &String) -> Option<&Vec<u8>> {
        if let Some(data) = self._store.get(key) {
            Some(&data._value)
        } else {
            None
        }
    }

    /*
     * pub fn _insert(&mut self, key: String, value: Vec<u8>) {}
     */

    /*
     * fn _process(&mut self, key: &String) {
     * }
     */

    pub fn purge(&mut self) {}
}
