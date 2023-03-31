use libp2p::PeerId;
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::time::Duration;

mod bandwidth;
mod latency;

use bandwidth::{Bandwidth, Bytes, BytesPerSecond, RequestId};
use latency::{Latency, Milliseconds};

const MAX_CAPACITY: usize = 100;

pub struct MeasurementManager {
    peers: LruCache<PeerId, PeerMeasurementManager>,
}

impl MeasurementManager {
    pub fn new() -> Self {
        Self {
            peers: LruCache::new(NonZeroUsize::new(MAX_CAPACITY).unwrap()),
        }
    }

    pub fn register_request(&mut self, peer_id: PeerId, request_id: RequestId, size: Bytes) {
        if !self.peers.contains(&peer_id) {
            self.peers.put(peer_id, PeerMeasurementManager::new());
        }
        let measurements = self.peers.get_mut(&peer_id).unwrap();
        measurements.bandwidth.register_request(request_id, size);
    }

    pub fn register_response(&mut self, peer_id: PeerId, request_id: RequestId, size: Bytes) {
        if let Some(measurements) = self.peers.get_mut(&peer_id) {
            measurements.bandwidth.register_response(request_id, size);
        }
    }

    pub fn register_rtt(&mut self, peer_id: PeerId, rtt: Duration) {
        if !self.peers.contains(&peer_id) {
            self.peers.put(peer_id, PeerMeasurementManager::new());
        }
        let measurements = self.peers.get_mut(&peer_id).unwrap();
        measurements.latency.register_rtt(rtt);
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
    pub bandwidth: Option<BytesPerSecond>,
    pub latency: Option<Milliseconds>,
}

struct PeerMeasurementManager {
    bandwidth: Bandwidth,
    latency: Latency,
}

impl PeerMeasurementManager {
    fn new() -> Self {
        Self {
            bandwidth: Bandwidth::new(),
            latency: Latency::new(),
        }
    }

    #[allow(dead_code)]
    fn get_measurements(&self) -> Option<Measurements> {
        let bandwidth = self.bandwidth.get_estimate();
        let latency = self.latency.get_estimate();
        if bandwidth.is_none() && latency.is_none() {
            None
        } else {
            Some(Measurements { bandwidth, latency })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    const EPSILON: f64 = 1e-6;

    #[test]
    fn test_one_request() {
        let peer_id = PeerId::random();
        let request_id = RequestId::from("1");
        let mut manager = MeasurementManager::new();
        manager.register_request(peer_id, request_id.clone(), 25_000);
        sleep(Duration::new(1, 0));
        manager.register_response(peer_id, request_id, 100_000);

        let measurements = manager.get_measurements();
        let measurement = measurements.get(&peer_id).unwrap();
        assert!((measurement.bandwidth.unwrap() - 125_000.0).abs() < EPSILON);
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
        assert!((measurement.bandwidth.unwrap() - 93750.0).abs() < EPSILON);
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

    #[test]
    fn test_latency_one_rtt() {
        let peer_id = PeerId::random();
        let mut manager = MeasurementManager::new();
        manager.register_rtt(peer_id, Duration::from_millis(300));

        let measurements = manager.get_measurements();
        let measurement = measurements.get(&peer_id).unwrap();
        assert!((measurement.latency.unwrap() - 150.0).abs() < EPSILON);
    }

    #[test]
    fn test_latency_two_rtt() {
        let peer_id = PeerId::random();
        let mut manager = MeasurementManager::new();
        manager.register_rtt(peer_id, Duration::from_millis(300));
        manager.register_rtt(peer_id, Duration::from_millis(400));

        let measurements = manager.get_measurements();
        let measurement = measurements.get(&peer_id).unwrap();
        assert!((measurement.latency.unwrap() - 175.0).abs() < EPSILON);
    }
}
