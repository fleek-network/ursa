use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::UNIX_EPOCH,
};

use anyhow::{bail, Context, Result};
use util::timer::now;

use super::lru::Lru;

struct Data {
    value: Vec<u8>,
    freq: usize,
    lru_k: usize,
    _tll: u128,
}

pub struct Tlrfu {
    store: HashMap<Arc<String>, Data>,
    freq: BTreeMap<usize, Lru<usize, Arc<String>>>, // shrinkable
    ttl: BTreeMap<u128, Arc<String>>,
    used_size: u64,
    _max_size: u64,
    _ttl_buf: u128,
}

impl Tlrfu {
    pub fn new(max_size: u64, ttl_buf: u128) -> Self {
        Self {
            store: HashMap::new(),
            freq: BTreeMap::new(),
            ttl: BTreeMap::new(),
            used_size: 0,
            _max_size: max_size,
            _ttl_buf: ttl_buf,
        }
    }

    fn _contains(&self, k: &String) -> bool {
        self.store.contains_key(k)
    }

    fn _is_size_exceeded(&self, bytes: u64) -> bool {
        self.used_size + bytes > self._max_size
    }

    pub async fn get(&mut self, k: &String) -> Result<Option<&Vec<u8>>> {
        if let Some(data) = self.store.get_mut(k) {
            let lru = self
                .freq
                .get_mut(&data.freq)
                .with_context(|| format!("[TLRFU]: Key: {k} not found at freq {}.", data.freq))?;
            let key = lru.remove(&data.lru_k).await.with_context(|| {
                format!(
                    "[TLRFU]: Failed to remove LRU key: {} not found at freq {}.",
                    data.lru_k, data.freq
                )
            })?;
            lru.is_empty().then(|| self.freq.remove(&data.freq));
            data.freq += 1;
            let lru = self.freq.entry(data.freq).or_insert(Lru::new(None));
            let lru_k = lru
                .get_tail_key()
                .map(|tail_key| *tail_key + 1)
                .unwrap_or(0);
            lru.insert(lru_k, key).await.with_context(|| {
                format!("[LRU]: Failed to insert LRU with key: {lru_k}, value: {k}")
            })?;
            data.lru_k = lru_k;
            Ok(Some(&data.value))
        } else {
            Ok(None)
        }
    }

    pub async fn _insert(&mut self, k: String, v: Vec<u8>) -> Result<()> {
        if self._contains(&k) {
            bail!("[TLRFU]: Key {k:?} existed while inserting.");
        }
        while self._is_size_exceeded(v.len() as u64) {
            let (&freq, lru) = self
                .freq
                .iter_mut()
                .next()
                .context("[TLRFU]: Freq is empty while deleting. Maybe size too big?")?;
            let key = lru.remove_head().await?.with_context(|| {
                format!("[LRU]: Failed to get deleted head key at freq: {freq}")
            })?;
            let data = self.store.remove(key.as_ref()).with_context(|| {
                format!("[TLRFU]: Key {key} not found at store while deleting.")
            })?;
            lru.is_empty().then(|| self.freq.remove(&freq));
            self.used_size -= data.value.len() as u64;
            self.ttl.remove(&data._tll);
        }
        let key = Arc::new(k);
        let lru = self.freq.entry(1).or_insert(Lru::new(None));
        let lru_k = lru
            .get_tail_key()
            .map(|tail_key| *tail_key + 1)
            .unwrap_or(0);
        lru.insert(lru_k, Arc::clone(&key)).await.with_context(|| {
            format!("[LRU]: Failed to insert LRU with key: {lru_k}, value: {key}")
        })?;
        self.used_size += v.len() as u64; // MAX = 2^64-1 bytes
        let tll = now()
            .duration_since(UNIX_EPOCH)
            .context("Failed to get system time from unix epoch")?
            .as_nanos()
            + self._ttl_buf;
        self.store.insert(
            Arc::clone(&key),
            Data {
                value: v,
                freq: 1,
                lru_k,
                _tll: tll,
            },
        );
        self.ttl.insert(tll, key);
        Ok(())
    }

    async fn _process_tll_clean_up(&mut self) -> Result<()> {
        loop {
            let (&ttl, key) = if let Some(next) = self.ttl.iter_mut().next() {
                next
            } else {
                return Ok(());
            };
            if ttl
                > now()
                    .duration_since(UNIX_EPOCH)
                    .context("Failed to get system time from unix epoch")?
                    .as_nanos()
            {
                return Ok(());
            }
            let data = self.store.remove(key.as_ref()).with_context(|| {
                format!("[TLRFU]: Key {key} not found at store while deleting.")
            })?;
            let lru = self
                .freq
                .get_mut(&data.freq)
                .with_context(|| format!("[TLRFU]: Key: {key} not found at freq {}.", data.freq))?;
            lru.remove(&data.lru_k).await.with_context(|| {
                format!(
                    "[TLRFU]: Failed to remove LRU key: {} not found at freq {}.",
                    data.lru_k, data.freq
                )
            })?;
            lru.is_empty().then(|| self.freq.remove(&data.freq));
            self.used_size -= data.value.len() as u64;
            self.ttl.remove(&data._tll);
        }
    }

    pub fn purge(&mut self) {
        self.store = HashMap::new();
        self.freq = BTreeMap::new();
        self.ttl = BTreeMap::new();
        self.used_size = 0;
    }
}

#[cfg(test)]
mod tests {
    use util::timer::mock_time::{clear_mock_time, set_mock_time};

    use super::*;

    #[tokio::test]
    async fn new() {
        let cache = Tlrfu::new(200_000_000, 0);
        assert_eq!(cache.store.len(), 0);
        assert_eq!(cache.freq.len(), 0);
        assert_eq!(cache.ttl.len(), 0);
        assert_eq!(cache.used_size, 0);
        assert_eq!(cache._max_size, 200_000_000);
        assert_eq!(cache._ttl_buf, 0);
    }

    #[tokio::test]
    async fn purge() {
        let mut cache = Tlrfu::new(200_000_000, 0);
        cache.purge();
        assert_eq!(cache.store.len(), 0);
        assert_eq!(cache.freq.len(), 0);
        assert_eq!(cache.ttl.len(), 0);
        assert_eq!(cache.used_size, 0);
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

        assert_eq!(cache.store.len(), 1);

        let data = cache.store.get(&"a".to_string()).unwrap();
        assert_eq!(data.value, &[0]);
        assert_eq!(data.freq, 1);
        assert_eq!(data.lru_k, 0);

        assert_eq!(cache.freq.len(), 1);

        let lru = cache.freq.get(&1).unwrap();
        assert_eq!(lru._len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"a".to_string());

        assert_eq!(cache.ttl.len(), 1);
        assert_eq!(cache.used_size, 1);
    }

    #[tokio::test]
    async fn insert_two() {
        let mut cache = Tlrfu::new(200_000_000, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        cache._insert("b".into(), vec![1]).await.unwrap();

        assert_eq!(cache.store.len(), 2);

        let data = cache.store.get(&"b".to_string()).unwrap();
        assert_eq!(data.value, &[1]);
        assert_eq!(data.freq, 1);
        assert_eq!(data.lru_k, 1);

        assert_eq!(cache.freq.len(), 1);

        let lru = cache.freq.get(&1).unwrap();
        assert_eq!(lru._len(), 2);
        assert_eq!(lru._get(&1).unwrap().as_ref(), &"b".to_string());

        assert_eq!(cache.ttl.len(), 2);
        assert_eq!(cache.used_size, 2);
    }

    #[tokio::test]
    async fn get_empty() {
        let mut cache = Tlrfu::new(200_000_000, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        assert!(cache.get(&"b".to_string()).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn get_one_with_one_bucket() {
        let mut cache = Tlrfu::new(200_000_000, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        assert_eq!(cache.get(&"a".to_string()).await.unwrap().unwrap(), &[0]);

        let data = cache.store.get(&"a".to_string()).unwrap();
        assert_eq!(data.value, &[0]);
        assert_eq!(data.freq, 2);
        assert_eq!(data.lru_k, 0);

        assert_eq!(cache.freq.len(), 1);

        let lru = cache.freq.get(&2).unwrap();
        assert_eq!(lru._len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"a".to_string());
    }

    #[tokio::test]
    async fn get_one_with_two_bucket() {
        let mut cache = Tlrfu::new(200_000_000, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        cache._insert("b".into(), vec![1]).await.unwrap();
        assert_eq!(cache.get(&"b".to_string()).await.unwrap().unwrap(), &[1]);

        let data = cache.store.get(&"b".to_string()).unwrap();
        assert_eq!(data.value, &[1]);
        assert_eq!(data.freq, 2);
        assert_eq!(data.lru_k, 0);

        assert_eq!(cache.freq.len(), 2);

        let lru = cache.freq.get(&2).unwrap();
        assert_eq!(lru._len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"b".to_string());
    }

    #[tokio::test]
    async fn get_two_with_one_bucket() {
        let mut cache = Tlrfu::new(200_000_000, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        assert_eq!(cache.get(&"a".to_string()).await.unwrap().unwrap(), &[0]);
        assert_eq!(cache.get(&"a".to_string()).await.unwrap().unwrap(), &[0]);

        let data = cache.store.get(&"a".to_string()).unwrap();
        assert_eq!(data.value, &[0]);
        assert_eq!(data.freq, 3);
        assert_eq!(data.lru_k, 0);

        assert_eq!(cache.freq.len(), 1);

        let lru = cache.freq.get(&3).unwrap();
        assert_eq!(lru._len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"a".to_string());
    }

    #[tokio::test]
    async fn get_two_with_two_bucket() {
        let mut cache = Tlrfu::new(200_000_000, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        cache._insert("b".into(), vec![1]).await.unwrap();
        assert_eq!(cache.get(&"b".to_string()).await.unwrap().unwrap(), &[1]);
        assert_eq!(cache.get(&"b".to_string()).await.unwrap().unwrap(), &[1]);

        let data = cache.store.get(&"b".to_string()).unwrap();
        assert_eq!(data.value, &[1]);
        assert_eq!(data.freq, 3);
        assert_eq!(data.lru_k, 0);

        assert_eq!(cache.freq.len(), 2);

        let lru = cache.freq.get(&3).unwrap();
        assert_eq!(lru._len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"b".to_string());
    }

    #[tokio::test]
    async fn insert_exceed_cap_with_one_bucket() {
        let mut cache = Tlrfu::new(2, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        cache._insert("b".into(), vec![1]).await.unwrap();
        cache._insert("c".into(), vec![2]).await.unwrap();

        assert_eq!(cache.store.len(), 2);

        assert!(cache.store.get(&"a".to_string()).is_none());

        let data = cache.store.get(&"b".to_string()).unwrap();
        assert_eq!(data.value, &[1]);
        assert_eq!(data.freq, 1);
        assert_eq!(data.lru_k, 1);

        let data = cache.store.get(&"c".to_string()).unwrap();
        assert_eq!(data.value, &[2]);
        assert_eq!(data.freq, 1);
        assert_eq!(data.lru_k, 2);

        assert_eq!(cache.freq.len(), 1);

        let lru = cache.freq.get(&1).unwrap();
        assert_eq!(lru._len(), 2);
        assert!(lru._get(&0).is_none());
        assert_eq!(lru._get(&1).unwrap().as_ref(), &"b".to_string());
        assert_eq!(lru._get(&2).unwrap().as_ref(), &"c".to_string());

        assert_eq!(cache.ttl.len(), 2);
        assert_eq!(cache.used_size, 2);
    }

    #[tokio::test]
    async fn insert_exceed_cap_with_two_bucket() {
        let mut cache = Tlrfu::new(2, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        cache._insert("b".into(), vec![1]).await.unwrap();

        assert_eq!(cache.get(&"a".to_string()).await.unwrap().unwrap(), &[0]);
        assert_eq!(cache.get(&"a".to_string()).await.unwrap().unwrap(), &[0]);
        assert_eq!(cache.get(&"b".to_string()).await.unwrap().unwrap(), &[1]);

        cache._insert("c".into(), vec![2]).await.unwrap();

        assert_eq!(cache.store.len(), 2);

        let data = cache.store.get(&"a".to_string()).unwrap();
        assert_eq!(data.value, &[0]);
        assert_eq!(data.freq, 3);
        assert_eq!(data.lru_k, 0);

        assert!(cache.store.get(&"b".to_string()).is_none());

        let data = cache.store.get(&"c".to_string()).unwrap();
        assert_eq!(data.value, &[2]);
        assert_eq!(data.freq, 1);
        assert_eq!(data.lru_k, 0);

        assert_eq!(cache.freq.len(), 2);

        let lru = cache.freq.get(&1).unwrap();
        assert_eq!(lru._len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"c".to_string());

        let lru = cache.freq.get(&3).unwrap();
        assert_eq!(lru._len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"a".to_string());

        assert_eq!(cache.ttl.len(), 2);
        assert_eq!(cache.used_size, 2);
    }

    #[tokio::test]
    async fn insert_exceed_cap_with_many_buckets_deleted() {
        let mut cache = Tlrfu::new(3, 0);
        cache._insert("a".into(), vec![0]).await.unwrap();
        cache._insert("b".into(), vec![1]).await.unwrap();
        cache._insert("c".into(), vec![2]).await.unwrap();
        cache._insert("d".into(), vec![3, 4, 5]).await.unwrap();

        assert_eq!(cache.store.len(), 1);

        assert!(cache.store.get(&"a".to_string()).is_none());
        assert!(cache.store.get(&"b".to_string()).is_none());
        assert!(cache.store.get(&"c".to_string()).is_none());
        let data = cache.store.get(&"d".to_string()).unwrap();
        assert_eq!(data.value, &[3, 4, 5]);
        assert_eq!(data.freq, 1);
        assert_eq!(data.lru_k, 0);

        assert_eq!(cache.freq.len(), 1);

        let lru = cache.freq.get(&1).unwrap();
        assert_eq!(lru._len(), 1);
        assert_eq!(lru._get(&0).unwrap().as_ref(), &"d".to_string());

        assert_eq!(cache.ttl.len(), 1);
        assert_eq!(cache.used_size, 3);
    }

    #[tokio::test]
    async fn process_tll_clean_up_successfully() {
        let mut cache = Tlrfu::new(3, 1_000_000_000);
        cache._insert("a".into(), vec![0]).await.unwrap();
        cache._insert("b".into(), vec![1]).await.unwrap();
        cache._insert("c".into(), vec![2]).await.unwrap();
        set_mock_time(
            now()
                .checked_add(std::time::Duration::from_nanos(1_000_000_000))
                .unwrap(),
        );
        cache._process_tll_clean_up().await.unwrap();
        assert_eq!(cache.store.len(), 0);
        assert_eq!(cache.freq.len(), 0);
        assert_eq!(cache.ttl.len(), 0);
        assert_eq!(cache.used_size, 0);
        clear_mock_time();
    }

    #[tokio::test]
    async fn process_tll_clean_up_skip() {
        let mut cache = Tlrfu::new(3, 1_000_000_000);
        cache._insert("a".into(), vec![0]).await.unwrap();
        cache._insert("b".into(), vec![1]).await.unwrap();
        cache._insert("c".into(), vec![2]).await.unwrap();
        set_mock_time(
            now()
                .checked_add(std::time::Duration::from_nanos(900_000_000))
                .unwrap(),
        );
        cache._process_tll_clean_up().await.unwrap();
        assert_eq!(cache.store.len(), 3);
        assert_eq!(cache.freq.len(), 1);
        assert_eq!(cache.ttl.len(), 3);
        assert_eq!(cache.used_size, 3);
        clear_mock_time();
    }
}
