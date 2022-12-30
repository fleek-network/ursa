use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CountingBloomFilter {
    buckets: Vec<u8>,
    num_hashes: usize,
}

impl CountingBloomFilter {
    pub fn new(num_elements: usize, fp_rate: f64) -> Self {
        let num_buckets = Self::calculate_num_buckets(num_elements, fp_rate);
        let num_hashes = Self::calculate_num_hashes(num_elements, num_buckets);

        CountingBloomFilter {
            buckets: vec![0; num_buckets],
            num_hashes,
        }
    }

    pub fn default() -> Self {
        CountingBloomFilter::new(10_000, 0.1)
    }

    pub fn insert<T: AsRef<[u8]>>(&mut self, value: &T) {
        for i in 0..self.num_hashes {
            let hash = fasthash::murmur3::hash32_with_seed(value, i as u32);
            let index = (hash % (self.buckets.len() as u32)) as usize;
            let count = self.buckets.get_mut(index).unwrap();
            *count = count.saturating_add(1);
        }
    }

    #[allow(dead_code)]
    pub fn contains<T: AsRef<[u8]>>(&self, value: &T) -> bool {
        for i in 0..self.num_hashes {
            let hash = fasthash::murmur3::hash32_with_seed(value, i as u32);
            let index = (hash % (self.buckets.len() as u32)) as usize;
            if self.buckets.get(index).unwrap() == &0u8 {
                return false;
            }
        }
        true
    }

    #[allow(dead_code)]
    pub fn remove<T: AsRef<[u8]>>(&mut self, value: &T) -> Result<()> {
        if !self.contains(value) {
            return Err(anyhow!("Element does not exist."));
        }
        for i in 0..self.num_hashes {
            let hash = fasthash::murmur3::hash32_with_seed(value, i as u32);
            let index = (hash % (self.buckets.len() as u32)) as usize;
            let count = self.buckets.get_mut(index).unwrap();
            *count = count.saturating_sub(1);
        }
        Ok(())
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|_| anyhow!("Failed to serialize bloom filter."))
    }

    pub fn deserialize(bytes: &[u8]) -> Result<CountingBloomFilter> {
        bincode::deserialize(bytes).map_err(|_| anyhow!("Failed to deserialize bloom filter."))
    }

    fn calculate_num_buckets(n: usize, fp_rate: f64) -> usize {
        ((-(n as f64) * fp_rate.ln()) / (2.0f64.ln().powf(2.0))).ceil() as usize
    }

    fn calculate_num_hashes(n: usize, m: usize) -> usize {
        (((m as f64) / (n as f64)) * 2.0f64.ln()).ceil() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_contains() {
        let mut filter = CountingBloomFilter::new(5, 0.01);
        filter.insert(&"abc");
        filter.insert(&"def");
        filter.insert(&"ghi");
        filter.insert(&"jkl");
        filter.insert(&"mnop");

        assert!(filter.contains(&"abc"));
        assert!(filter.contains(&"def"));
        assert!(filter.contains(&"ghi"));
        assert!(filter.contains(&"jkl"));
        assert!(filter.contains(&"mnop"));

        assert!(!filter.contains(&"xyz"));
        assert!(!filter.contains(&"test"));
        assert!(!filter.contains(&"hallo"));
        assert!(!filter.contains(&"1234"));
    }

    #[test]
    fn test_remove() {
        let mut filter = CountingBloomFilter::new(5, 0.01);
        filter.insert(&"xyz");
        filter.insert(&"fgh");
        filter.insert(&"hjz");
        filter.insert(&"dfgh");
        filter.insert(&"oiuz");

        filter.remove(&"xyz").unwrap();
        assert!(!filter.contains(&"xyz"));

        filter.remove(&"fgh").unwrap();
        assert!(!filter.contains(&"fgh"));

        filter.remove(&"hjz").unwrap();
        assert!(!filter.contains(&"hjz"));

        filter.remove(&"dfgh").unwrap();
        assert!(!filter.contains(&"dfgh"));

        filter.remove(&"oiuz").unwrap();
        assert!(!filter.contains(&"oizu"));
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut filter = CountingBloomFilter::new(10, 0.01);
        filter.insert(&"abc");
        filter.insert(&"def");
        filter.insert(&"ghi");

        let bytes = filter.serialize().unwrap();
        let filter = CountingBloomFilter::deserialize(&bytes).unwrap();

        assert!(filter.contains(&"abc"));
        assert!(filter.contains(&"def"));
        assert!(filter.contains(&"ghi"));
    }
}
