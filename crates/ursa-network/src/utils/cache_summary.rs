use anyhow::{anyhow, Result};
use rand::SeedableRng;
use scalable_cuckoo_filter::{ScalableCuckooFilter, ScalableCuckooFilterBuilder};
use serde::{Deserialize, Serialize};
use siphasher::sip::SipHasher13;

#[derive(Serialize, Deserialize, Debug)]
pub struct CacheSummary {
    filter: ScalableCuckooFilter<[u8], SipHasher13>,
}

impl CacheSummary {
    pub fn new(initial_capacity: usize, fp_rate: f64) -> CacheSummary {
        CacheSummary {
            filter: ScalableCuckooFilterBuilder::new()
                .initial_capacity(initial_capacity)
                .false_positive_probability(fp_rate)
                .rng(SeedableRng::from_entropy())
                .finish(),
        }
    }

    pub fn default() -> Self {
        Self::new(10_000, 0.1)
    }

    pub fn insert<T: AsRef<[u8]>>(&mut self, value: T) {
        self.filter.insert(value.as_ref());
    }

    pub fn contains<T: AsRef<[u8]>>(&self, value: T) -> bool {
        self.filter.contains(value.as_ref())
    }

    #[allow(dead_code)]
    pub fn remove<T: AsRef<[u8]>>(&mut self, value: T) {
        self.filter.remove(value.as_ref());
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|_| anyhow!("Failed to serialize cache summary."))
    }

    pub fn deserialize(bytes: &[u8]) -> Result<CacheSummary> {
        bincode::deserialize(bytes).map_err(|_| anyhow!("Failed to deserialize cache summary."))
    }
}

impl Clone for CacheSummary {
    fn clone(&self) -> Self {
        CacheSummary {
            filter: self.filter.clone(),
        }
    }
}

impl PartialEq for CacheSummary {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
    }
}

impl Eq for CacheSummary {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_contains() {
        let mut filter = CacheSummary::new(10, 0.01);
        filter.insert(b"abc");
        filter.insert(b"def");
        filter.insert(b"ghi");
        filter.insert(b"jkl");
        filter.insert(b"mnop");

        assert!(filter.contains(b"abc"));
        assert!(filter.contains(b"def"));
        assert!(filter.contains(b"ghi"));
        assert!(filter.contains(b"jkl"));
        assert!(filter.contains(b"mnop"));

        assert!(!filter.contains(b"xyz"));
        assert!(!filter.contains(b"test"));
        assert!(!filter.contains(b"hallo"));
        assert!(!filter.contains(b"1234"));
    }

    #[test]
    fn test_remove() {
        let mut filter = CacheSummary::new(5, 0.01);
        filter.insert(b"xyz");
        filter.insert(b"fgh");
        filter.insert(b"hjz");
        filter.insert(b"dfgh");
        filter.insert(b"oiuz");

        filter.remove(b"xyz");
        assert!(!filter.contains(b"xyz"));

        filter.remove(b"fgh");
        assert!(!filter.contains(b"fgh"));

        filter.remove(b"hjz");
        assert!(!filter.contains(b"hjz"));

        filter.remove(b"dfgh");
        assert!(!filter.contains(b"dfgh"));

        filter.remove(b"oiuz");
        assert!(!filter.contains(b"oizu"));
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut filter = CacheSummary::new(10, 0.01);
        filter.insert(b"abc");
        filter.insert(b"def");
        filter.insert(b"ghi");

        let bytes = filter.serialize().unwrap();
        let filter = CacheSummary::deserialize(&bytes).unwrap();

        assert!(filter.contains(b"abc"));
        assert!(filter.contains(b"def"));
        assert!(filter.contains(b"ghi"));
    }
}
