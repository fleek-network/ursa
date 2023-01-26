mod lru;

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::cache::tlrfu::lru::Lru;
use anyhow::{bail, Context, Result};

struct Data<T: ByteSize> {
    value: Arc<T>,
    freq: usize,
    lru_k: usize,
    ttl: u128,
}

pub struct Tlrfu<T: ByteSize> {
    store: HashMap<Arc<String>, Data<T>>,
    freq: BTreeMap<usize, Lru<usize, Arc<String>>>, // shrinkable
    ttl: BTreeMap<u128, Arc<String>>,
    used_size: u64,
    max_size: u64,
    ttl_buf: u128,
}

pub trait ByteSize {
    fn len(&self) -> usize;
}

#[cfg(not(test))]
pub fn now() -> SystemTime {
    SystemTime::now()
}

impl<T: ByteSize> Tlrfu<T> {
    pub fn new(max_size: u64, ttl_buf: u128) -> Self {
        Self {
            store: HashMap::new(),
            freq: BTreeMap::new(),
            ttl: BTreeMap::new(),
            used_size: 0,
            max_size,
            ttl_buf,
        }
    }

    pub fn contains(&self, k: &String) -> bool {
        self.store.contains_key(k)
    }

    fn is_size_exceeded(&self, bytes: u64) -> bool {
        self.used_size + bytes > self.max_size
    }

    pub fn dirty_get(&self, k: &String) -> Option<&Arc<T>> {
        self.store.get(k).map(|data| &data.value)
    }

    pub async fn get(&mut self, k: &String) -> Result<Option<&Arc<T>>> {
        if let Some(data) = self.store.get_mut(k) {
            let lru = self
                .freq
                .get_mut(&data.freq)
                .with_context(|| format!("[TLRFU]: Key: {k} not found at freq {}", data.freq))?;
            let key = lru.remove(&data.lru_k).await.with_context(|| {
                format!(
                    "[TLRFU]: Failed to remove LRU key: {} not found at freq {}",
                    data.lru_k, data.freq
                )
            })?;
            lru.is_empty().then(|| self.freq.remove(&data.freq));
            data.freq += 1;
            let lru = self.freq.entry(data.freq).or_insert_with(|| Lru::new(None));
            let lru_k = lru
                .get_tail_key()
                .map(|tail_key| *tail_key + 1)
                .unwrap_or(0);
            lru.insert(lru_k, key).await.with_context(|| {
                format!("[LRU]: Failed to insert LRU with key: {lru_k}, value: {k}")
            })?;
            data.lru_k = lru_k;
            let key = self
                .ttl
                .remove(&data.ttl)
                .with_context(|| format!("[TLRFU]: Key not found when delete ttl: {}", data.ttl))?;
            data.ttl = now()
                .duration_since(UNIX_EPOCH)
                .context("Failed to get system time from unix epoch")?
                .as_nanos()
                + self.ttl_buf;
            self.ttl.insert(data.ttl, key);
            Ok(Some(&data.value))
        } else {
            Ok(None)
        }
    }

    pub async fn insert(&mut self, k: String, v: Arc<T>) -> Result<()> {
        if self.contains(&k) {
            bail!("[TLRFU]: Key {k:?} existed while inserting");
        }
        while self.is_size_exceeded(v.len() as u64) {
            let (&freq, lru) = self
                .freq
                .iter_mut()
                .next()
                .context("[TLRFU]: Freq is empty while deleting. Maybe size too big?")?;
            let key = lru.remove_head().await?.with_context(|| {
                format!("[LRU]: Failed to get deleted head key at freq: {freq}")
            })?;
            let data = self
                .store
                .remove(key.as_ref())
                .with_context(|| format!("[TLRFU]: Key {key} not found at store while deleting"))?;
            lru.is_empty().then(|| self.freq.remove(&freq));
            self.used_size -= data.value.len() as u64;
            self.ttl.remove(&data.ttl);
        }
        let key = Arc::new(k);
        let lru = self.freq.entry(1).or_insert_with(|| Lru::new(None));
        let lru_k = lru
            .get_tail_key()
            .map(|tail_key| *tail_key + 1)
            .unwrap_or(0);
        lru.insert(lru_k, Arc::clone(&key)).await.with_context(|| {
            format!("[LRU]: Failed to insert LRU with key: {lru_k}, value: {key}")
        })?;
        self.used_size += v.len() as u64; // MAX = 2^64-1 bytes
        let ttl = now()
            .duration_since(UNIX_EPOCH)
            .context("Failed to get system time from unix epoch")?
            .as_nanos()
            + self.ttl_buf;
        self.store.insert(
            Arc::clone(&key),
            Data {
                value: v,
                freq: 1,
                lru_k,
                ttl,
            },
        );
        self.ttl.insert(ttl, key);
        Ok(())
    }

    pub async fn process_ttl_clean_up(&mut self) -> Result<usize> {
        let mut count = 0;
        loop {
            let (&ttl, key) = if let Some(next) = self.ttl.iter_mut().next() {
                next
            } else {
                return Ok(count);
            };
            if ttl
                > now()
                    .duration_since(UNIX_EPOCH)
                    .context("Failed to get system time from unix epoch")?
                    .as_nanos()
            {
                return Ok(count);
            }
            let data = self
                .store
                .remove(key.as_ref())
                .with_context(|| format!("[TLRFU]: Key {key} not found at store while deleting"))?;
            let lru = self
                .freq
                .get_mut(&data.freq)
                .with_context(|| format!("[TLRFU]: Key: {key} not found at freq {}", data.freq))?;
            lru.remove(&data.lru_k).await.with_context(|| {
                format!(
                    "[TLRFU]: Failed to remove LRU key: {} not found at freq {}",
                    data.lru_k, data.freq
                )
            })?;
            lru.is_empty().then(|| self.freq.remove(&data.freq));
            self.used_size -= data.value.len() as u64;
            self.ttl.remove(&data.ttl);
            count += 1;
        }
    }
}
