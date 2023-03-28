use libp2p::PeerId;
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

type Bytes = u128;
type BytesPerSecond = f64;
type RequestId = String;

const MAX_CAPACITY: usize = 100;

pub struct MeasurementManager {
    peers: LruCache<PeerId, PeerMeasurement>,
}

impl MeasurementManager {
    pub fn new() -> Self {
        Self {
            peers: LruCache::new(NonZeroUsize::new(MAX_CAPACITY).unwrap()),
        }
    }

    pub fn register_request(&mut self, peer_id: PeerId, request_id: RequestId, size: Bytes) {
        if !self.peers.contains(&peer_id) {
            self.peers.put(peer_id, PeerMeasurement::new());
        }
        let measurements = self.peers.get_mut(&peer_id).unwrap();
        measurements.bandwidth.register_request(request_id, size);
    }

    pub fn register_response(&mut self, peer_id: PeerId, request_id: RequestId, size: Bytes) {
        if let Some(measurements) = self.peers.get_mut(&peer_id) {
            measurements.bandwidth.register_response(request_id, size);
        }
    }

    #[allow(dead_code)]
    pub fn get_measurements(&self) -> HashMap<PeerId, Measurements> {
        self.peers
            .iter()
            .filter_map(|(peer_id, manager)| manager.get_measurements().map(|m| (*peer_id, m)))
            .collect()
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.peers.clear();
    }
}

impl Default for MeasurementManager {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
pub struct Measurements {
    pub bandwidth: BytesPerSecond,
}

struct PeerMeasurement {
    bandwidth: Bandwidth,
    // TODO(matthias): add latency and uptime measurements
}

impl PeerMeasurement {
    fn new() -> Self {
        Self {
            bandwidth: Bandwidth::new(),
        }
    }

    #[allow(dead_code)]
    fn get_measurements(&self) -> Option<Measurements> {
        let bandwidth = self.bandwidth.get_estimate()?;
        Some(Measurements { bandwidth })
    }
}

#[derive(Clone)]
struct Bandwidth {
    requests: HashMap<RequestId, Request>,
    sum: BytesPerSecond,
    count: u64,
}

impl Bandwidth {
    fn new() -> Self {
        Self {
            requests: HashMap::new(),
            sum: 0.0,
            count: 0,
        }
    }

    fn register_request(&mut self, request_id: RequestId, size: Bytes) {
        self.requests.insert(request_id, Request::new(size));
    }

    fn register_response(&mut self, request_id: RequestId, size: Bytes) {
        if let Some(request) = self.requests.remove(&request_id) {
            let total_size = request.size + size;
            let duration = request.duration().as_secs();
            if duration > 0 {
                self.sum += (total_size as f64) / (duration as f64);
                self.count += 1;
            }
        }
    }

    #[allow(dead_code)]
    fn get_estimate(&self) -> Option<BytesPerSecond> {
        if self.count > 0 {
            Some(self.sum / (self.count as f64))
        } else {
            None
        }
    }
}

#[derive(Clone)]
struct Request {
    instant: Instant,
    size: Bytes,
}

impl Request {
    fn new(size: Bytes) -> Self {
        Self {
            instant: Instant::now(),
            size,
        }
    }

    fn duration(&self) -> Duration {
        self.instant.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    const EPSILON: f64 = 1e-6;

    #[test]
    fn test_basic() {
        let peer_id = PeerId::random();
        let request_id = RequestId::from("1");
        let mut manager = MeasurementManager::new();
        manager.register_request(peer_id, request_id.clone(), 25_000);
        sleep(Duration::new(1, 0));
        manager.register_response(peer_id, request_id, 100_000);

        let measurements = manager.get_measurements();
        let measurement = measurements.get(&peer_id).unwrap();
        assert!((measurement.bandwidth - 125_000.0).abs() < EPSILON);
    }

    #[test]
    fn test_two_requests() {
        let peer_id = PeerId::random();
        let request_id = RequestId::from("1");
        let mut manager = MeasurementManager::new();
        manager.register_request(peer_id, request_id.clone(), 0);
        sleep(Duration::new(1, 0));
        manager.register_response(peer_id, request_id, 125_000);

        let request_id = RequestId::from("2");
        manager.register_request(peer_id, request_id.clone(), 62_500);
        sleep(Duration::new(2, 0));
        manager.register_response(peer_id, request_id, 62_500);

        let measurements = manager.get_measurements();
        let measurement = measurements.get(&peer_id).unwrap();
        assert!((measurement.bandwidth - 93750.0).abs() < EPSILON);
    }

    #[test]
    fn test_missing_request() {
        let peer_id = PeerId::random();
        let request_id = RequestId::from("1");
        let mut manager = MeasurementManager::new();
        manager.register_response(peer_id, request_id, 100_000);

        let measurements = manager.get_measurements();
        let measurement = measurements.get(&peer_id);
        assert!(measurement.is_none());
    }

    #[test]
    fn test_missing_response() {
        let peer_id = PeerId::random();
        let request_id = RequestId::from("1");
        let mut manager = MeasurementManager::new();
        manager.register_request(peer_id, request_id, 100_000);

        let measurements = manager.get_measurements();
        let measurement = measurements.get(&peer_id);
        assert!(measurement.is_none());
    }
}
