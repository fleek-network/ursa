mod lru;

use std::{
    cmp::{Ordering, PartialEq},
    collections::{BinaryHeap, HashMap},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Deserialize, Serialize)]
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
#[derive(PartialEq, Eq, Deserialize, Serialize)]
struct MinFrequency {
    key: String,
    count: u64,
}
impl PartialOrd for MinFrequency {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.count.partial_cmp(&self.count)
    }
}
impl Ord for MinFrequency {
    fn cmp(&self, other: &Self) -> Ordering {
        other.count.cmp(&self.count)
    }
}

#[derive(Deserialize, Serialize)]
pub struct TLRFUCache {
    stores: HashMap<String, Vec<u8>>,
    used_size: u64,
    ttls: BinaryHeap<MinTTL>,
    frequencies: BinaryHeap<MinFrequency>,
    sizes: HashMap<String, u64>,
    max_size: u64,
    ttl_buf: u128,
}

impl TLRFUCache {
    pub fn new(max_size: u64, ttl_buf: u128) -> Self {
        Self {
            stores: HashMap::new(),
            used_size: 0,
            ttls: BinaryHeap::new(),
            frequencies: BinaryHeap::new(),
            sizes: HashMap::new(),
            max_size,
            ttl_buf,
        }
    }

    pub fn _is_size_exceeded(&self) -> bool {
        self.used_size >= self.max_size
    }

    pub fn _is_ttl_elapsed(&self) -> bool {
        false
        /*
         * if let Some(_entry) = self.ttls.peek() {
         *     true
         * } else {
         *     false
         * }
         */
    }

    pub fn _get(&self, key: &String) -> Option<&Vec<u8>> {
        self.stores.get(key)
    }

    pub fn _insert(&mut self, key: String, value: Vec<u8>) {
        self.used_size += value.len() as u64;
        if let Ok(ms) = SystemTime::now().duration_since(UNIX_EPOCH) {
            self.ttls.push(MinTTL {
                key: key.clone(),
                ttl: ms.as_millis() + self.ttl_buf,
            })
        }
        self.frequencies.push(MinFrequency {
            key: key.clone(),
            count: 0,
        });
        self.sizes.insert(key.clone(), value.len() as u64);
        self.stores.insert(key, value);
    }

    fn _process(&mut self, key: &String) -> Option<Vec<u8>> {
        if let Some(old) = self.stores.remove(key) {
            self.used_size -= old.len() as u64;
            self.ttls = BinaryHeap::new();
            self.frequencies = BinaryHeap::new();
            self.sizes.remove(key);
            Some(old)
        } else {
            None
        }
    }

    pub fn purge(&mut self) {
        self.stores = HashMap::new();
        self.used_size = 0;
        self.ttls = BinaryHeap::new();
        self.frequencies = BinaryHeap::new();
        self.sizes = HashMap::new();
    }
}
