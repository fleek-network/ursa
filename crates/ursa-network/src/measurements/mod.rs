use libp2p::PeerId;
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
#[cfg(test)]
use std::thread::sleep;
use std::time::Duration;

mod bandwidth;
mod latency;
mod types;
mod uptime;

use bandwidth::Bandwidth;
use latency::Latency;

use crate::measurements::uptime::Uptime;
use types::{Bytes, RequestId};

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
            self.peers
                .put(peer_id, PeerMeasurementManager::new(peer_id));
        }
        let measurements = self.peers.get_mut(&peer_id).unwrap();
        measurements.bandwidth.register_request(request_id, size);
    }

    pub fn register_response(&mut self, peer_id: PeerId, request_id: RequestId, size: Bytes) {
        if let Some(measurements) = self.peers.get_mut(&peer_id) {
            measurements.bandwidth.register_response(request_id, size);
        }
    }

    pub fn register_ping(&mut self, peer_id: PeerId, rtt: Duration) {
        if !self.peers.contains(&peer_id) {
            self.peers
                .put(peer_id, PeerMeasurementManager::new(peer_id));
        }
        let measurements = self.peers.get_mut(&peer_id).unwrap();
        measurements.latency.register_rtt(rtt);
        measurements.uptime.register_ping();
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

    #[cfg(test)]
    fn add_dummy_bandwidth(&mut self, peer_id: PeerId) {
        let request_id = RequestId::from("1");
        self.register_request(peer_id, request_id.clone(), 0);
        sleep(Duration::new(1, 0));
        self.register_response(peer_id, request_id, 0);
    }
}

impl Default for MeasurementManager {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
pub struct Measurements {
    pub peer_id: String,
    pub bandwidth: u64, // bytes/s
    pub latency: u32,   // milliseconds
    pub uptime: u128,   // milliseconds
}

struct PeerMeasurementManager {
    peer_id: PeerId,
    bandwidth: Bandwidth,
    latency: Latency,
    uptime: Uptime,
}

impl PeerMeasurementManager {
    fn new(peer_id: PeerId) -> Self {
        Self {
            peer_id,
            bandwidth: Bandwidth::new(),
            latency: Latency::new(),
            uptime: Uptime::new(),
        }
    }

    #[allow(dead_code)]
    fn get_measurements(&self) -> Option<Measurements> {
        let bandwidth = self.bandwidth.get_estimate()? as u64;
        let latency = self.latency.get_estimate()? as u32;
        let uptime = self.uptime.get_estimate()? as u128;
        Some(Measurements {
            peer_id: self.peer_id.to_base58(),
            bandwidth,
            latency,
            uptime,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;
    use types::RequestId;

    #[test]
    fn test_one_request() {
        let peer_id = PeerId::random();
        let request_id = RequestId::from("1");
        let mut manager = MeasurementManager::new();

        // Register ping so that `measurements.get(&peer_id)` does not
        // return `Ǹone` because there are no uptime and latency measurements
        manager.register_ping(peer_id, Duration::from_millis(100));

        manager.register_request(peer_id, request_id.clone(), 25_000);
        sleep(Duration::new(1, 0));
        manager.register_response(peer_id, request_id, 100_000);

        let measurements = manager.get_measurements();
        let measurement = measurements.get(&peer_id).unwrap();
        //assert!((measurement.bandwidth.unwrap() - 125_000.0).abs() < EPSILON);
        assert_eq!(measurement.bandwidth, 125_000);
    }

    #[test]
    fn test_two_requests() {
        let peer_id = PeerId::random();
        let request_id = RequestId::from("1");
        let mut manager = MeasurementManager::new();

        // Register ping so that `measurements.get(&peer_id)` does not
        // return `Ǹone` because there are no uptime and latency measurements
        manager.register_ping(peer_id, Duration::from_millis(100));

        manager.register_request(peer_id, request_id.clone(), 0);
        sleep(Duration::new(1, 0));
        manager.register_response(peer_id, request_id, 125_000);

        let request_id = RequestId::from("2");
        manager.register_request(peer_id, request_id.clone(), 62_500);
        sleep(Duration::new(2, 0));
        manager.register_response(peer_id, request_id, 62_500);

        let measurements = manager.get_measurements();
        let measurement = measurements.get(&peer_id).unwrap();
        //assert!((measurement.bandwidth.unwrap() - 93750.0).abs() < EPSILON);
        assert_eq!(measurement.bandwidth, 93750);
    }

    #[test]
    fn test_missing_request() {
        let peer_id = PeerId::random();
        let request_id = RequestId::from("1");
        let mut manager = MeasurementManager::new();

        // Register ping to ensure that `measurements.get(&peer_id)`
        // only returns `None` because of the missing bandwidth
        manager.register_ping(peer_id, Duration::from_millis(100));

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

        // Register ping to ensure that `measurements.get(&peer_id)`
        // only returns `None` because of the missing bandwidth
        manager.register_ping(peer_id, Duration::from_millis(100));

        manager.register_request(peer_id, request_id, 100_000);

        let measurements = manager.get_measurements();
        let measurement = measurements.get(&peer_id);
        assert!(measurement.is_none());
    }

    #[test]
    fn test_latency_one_ping() {
        let peer_id = PeerId::random();
        let mut manager = MeasurementManager::new();

        // Add dummy bandwidth so that `measurements.get(&peer_id)` does not
        // return `Ǹone` because there are no bandwidth measurements
        manager.add_dummy_bandwidth(peer_id);

        manager.register_ping(peer_id, Duration::from_millis(300));

        let measurements = manager.get_measurements();
        let measurement = measurements.get(&peer_id).unwrap();
        //assert!((measurement.latency.unwrap() - 150.0).abs() < EPSILON);
        assert_eq!(measurement.latency, 150);
    }

    #[test]
    fn test_latency_two_pings() {
        let peer_id = PeerId::random();
        let mut manager = MeasurementManager::new();

        // Add dummy bandwidth so that `measurements.get(&peer_id)` does not
        // return `Ǹone` because there are no bandwidth measurements
        manager.add_dummy_bandwidth(peer_id);

        manager.register_ping(peer_id, Duration::from_millis(300));
        manager.register_ping(peer_id, Duration::from_millis(400));

        let measurements = manager.get_measurements();
        let measurement = measurements.get(&peer_id).unwrap();
        //assert!((measurement.latency.unwrap() - 175.0).abs() < EPSILON);
        assert_eq!(measurement.latency, 175);
    }

    #[test]
    fn test_uptime_one_ping() {
        let peer_id = PeerId::random();
        let mut manager = MeasurementManager::new();

        // Add dummy bandwidth so that `measurements.get(&peer_id)` does not
        // return `Ǹone` because there are no bandwidth measurements
        manager.add_dummy_bandwidth(peer_id);

        manager.register_ping(peer_id, Duration::from_millis(100));

        let measurements = manager.get_measurements();
        let measurement = measurements.get(&peer_id).unwrap();
        //assert_eq!(measurement.uptime.unwrap(), 0.0);
        assert_eq!(measurement.uptime, 0);
    }

    #[test]
    fn test_uptime_two_pings() {
        let peer_id = PeerId::random();
        let mut manager = MeasurementManager::new();

        // Add dummy bandwidth so that `measurements.get(&peer_id)` does not
        // return `Ǹone` because there are no bandwidth measurements
        manager.add_dummy_bandwidth(peer_id);

        manager.register_ping(peer_id, Duration::from_millis(100));
        sleep(Duration::new(1, 0));
        manager.register_ping(peer_id, Duration::from_millis(100));

        let measurements = manager.get_measurements();
        let measurement = measurements.get(&peer_id).unwrap();
        let uptime = measurement.uptime;
        // Relaxed timing requirements to make test work on macos
        //assert!((1000.0..2000.0).contains(&uptime))
        assert!((1000..2000).contains(&uptime))
    }

    #[test]
    fn test_uptime_timeout() {
        let peer_id = PeerId::random();
        let mut manager = MeasurementManager::new();

        // Add dummy bandwidth so that `measurements.get(&peer_id)` does not
        // return `Ǹone` because there are no bandwidth measurements
        manager.add_dummy_bandwidth(peer_id);

        manager.register_ping(peer_id, Duration::from_millis(100));
        sleep(Duration::new(1, 0));
        manager.register_ping(peer_id, Duration::from_millis(100));
        // `MAX_TIME_BETWEEN_PINGS` is set to 2 seconds for test mode
        sleep(Duration::new(3, 0));

        let measurements = manager.get_measurements();
        let measurement = measurements.get(&peer_id).unwrap();
        //assert_eq!(measurement.uptime.unwrap(), 0.0);
        assert_eq!(measurement.uptime, 0);
    }
}
