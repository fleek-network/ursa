use std::time::Duration;

pub type Milliseconds = f64;

#[derive(Clone)]
pub struct Latency {
    sum: Milliseconds,
    count: u64,
}

impl Latency {
    pub fn new() -> Self {
        Self { sum: 0.0, count: 0 }
    }

    pub fn register_rtt(&mut self, rtt: Duration) {
        let rtt_millis = rtt.as_millis() as f64;
        if rtt_millis > 0.0 {
            self.sum += rtt_millis / 2.0;
            self.count += 1;
        }
    }

    #[allow(dead_code)]
    pub fn get_estimate(&self) -> Option<Milliseconds> {
        if self.count > 0 {
            Some(self.sum / (self.count as f64))
        } else {
            None
        }
    }
}
