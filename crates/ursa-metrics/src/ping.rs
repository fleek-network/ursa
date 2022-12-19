use crate::Recorder;
use libp2p::ping::{Event, Failure, Success};
use metrics::{histogram, increment_counter, Label};

impl Recorder for Event {
    fn record(&self) {
        match &self.result {
            Ok(Success::Pong) => {
                increment_counter!("ping_pong_received");
            }
            Ok(Success::Ping { rtt }) => {
                histogram!("ping_rtt", rtt.as_secs_f64());
            }
            Err(f) => {
                increment_counter!("ping_error", vec![failure_label(f)]);
            }
        }
    }
}

fn failure_label(f: &Failure) -> Label {
    match f {
        Failure::Timeout => Label::new("failure", "timeout"),
        Failure::Unsupported => Label::new("failure", "unsupported"),
        Failure::Other { .. } => Label::new("failure", "other"),
    }
}
