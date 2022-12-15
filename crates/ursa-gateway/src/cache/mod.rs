use std::{
    collections::{BinaryHeap, HashMap},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct LFUCacheTLL {
    stores: HashMap<String, Vec<u8>>,
    max_size: u64,
    used_size: u64,
    ttls: BinaryHeap<(String, u128)>,
    frequencies: BinaryHeap<(String, u64)>,
    sizes: HashMap<String, u64>,
}

impl LFUCacheTLL {
    pub fn new(max_size: u64) -> Self {
        Self {
            stores: HashMap::new(),
            max_size,
            used_size: 0,
            ttls: BinaryHeap::new(),
            frequencies: BinaryHeap::new(),
            sizes: HashMap::new(),
        }
    }

    pub fn _get(&self, key: &String) -> Option<&Vec<u8>> {
        self.stores.get(key)
    }

    pub fn _insert(&mut self, key: String, value: Vec<u8>) {
        self.used_size += value.len() as u64;
        if let Ok(ms) = SystemTime::now().duration_since(UNIX_EPOCH) {
            self.ttls.push((key.clone(), ms.as_millis()))
        }
        self.frequencies.push((key.clone(), 0));
        self.sizes.insert(key.clone(), value.len() as u64);
        self.stores.insert(key, value);
    }

    fn _remove(&mut self, key: &String) -> Option<Vec<u8>> {
        self.stores.remove(key)
    }

    pub fn purge(&mut self) {
        self.stores = HashMap::new();
        self.used_size = 0;
        self.ttls = BinaryHeap::new();
        self.frequencies = BinaryHeap::new();
        self.sizes = HashMap::new();
    }
}
