use crate::measurements::types::Milliseconds;
use std::time::{Duration, Instant};

#[cfg(not(test))]
const MAX_TIME_BETWEEN_PINGS: Duration = Duration::from_secs(60);
#[cfg(test)]
const MAX_TIME_BETWEEN_PINGS: Duration = Duration::from_secs(2);

pub struct Uptime {
    last_ping: Option<Instant>,
    uptime: Milliseconds,
}

impl Uptime {
    pub fn new() -> Self {
        Self {
            last_ping: None,
            uptime: 0.0,
        }
    }

    pub fn register_ping(&mut self) {
        let now = Instant::now();
        if let Some(last_ping) = self.last_ping {
            let elapsed = last_ping.elapsed();
            if elapsed < MAX_TIME_BETWEEN_PINGS {
                self.uptime += elapsed.as_millis() as f64;
            } else {
                self.uptime = 0.0;
            }
        }
        self.last_ping = Some(now);
    }

    pub fn get_estimate(&self) -> Option<Milliseconds> {
        match self.last_ping {
            Some(last_ping) => {
                let elapsed = last_ping.elapsed();
                if elapsed < MAX_TIME_BETWEEN_PINGS {
                    Some(self.uptime)
                } else {
                    Some(0.0)
                }
            }
            None => None,
        }
    }
}
