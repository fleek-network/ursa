use crate::Recorder;
use libp2p_ping::Event;
use metrics::{histogram, increment_counter, Label};

impl Recorder for Event {
    fn record(&self) {
        match &self.result {
            Ok(libp2p_ping::Success::Pong) => {
                increment_counter!("ping_pong_received");
            }
            Ok(libp2p_ping::Success::Ping { rtt }) => {
                histogram!("ping_rtt", rtt.as_secs_f64());
            }
            Err(f) => {
                increment_counter!("ping_error", vec![Failure::from(f).into()]);
            }
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
enum Failure {
    Timeout,
    Unsupported,
    Other,
}

impl From<&libp2p_ping::Failure> for Failure {
    fn from(failure: &libp2p_ping::Failure) -> Self {
        match failure {
            libp2p_ping::Failure::Timeout => Failure::Timeout,
            libp2p_ping::Failure::Unsupported => Failure::Unsupported,
            libp2p_ping::Failure::Other { .. } => Failure::Other,
        }
    }
}

impl From<Failure> for Label {
    fn from(failure: Failure) -> Self {
        match failure {
            Failure::Timeout => Label::new("failure", "timeout"),
            Failure::Unsupported => Label::new("failure", "unsupported"),
            Failure::Other => Label::new("failure", "other"),
        }
    }
}
