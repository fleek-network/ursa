use std::collections::HashMap;
use std::time::{Duration, Instant};

pub type Bytes = u128;
pub type BytesPerSecond = f64;
pub type RequestId = String;

#[derive(Clone)]
pub struct Bandwidth {
    requests: HashMap<RequestId, Request>,
    sum: BytesPerSecond,
    count: u64,
}

impl Bandwidth {
    pub fn new() -> Self {
        Self {
            requests: HashMap::new(),
            sum: 0.0,
            count: 0,
        }
    }

    pub fn register_request(&mut self, request_id: RequestId, size: Bytes) {
        self.requests.insert(request_id, Request::new(size));
    }

    pub fn register_response(&mut self, request_id: RequestId, size: Bytes) {
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
    pub fn get_estimate(&self) -> Option<BytesPerSecond> {
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
