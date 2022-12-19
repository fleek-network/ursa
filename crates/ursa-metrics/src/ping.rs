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
                increment_counter!("ping_error", vec![Failure::from(f).into()]);
            }
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
enum FailureLabel {
    Timeout,
    Unsupported,
    Other,
}

impl From<&Failure> for FailureLabel {
    fn from(failure: &Failure) -> Self {
        match failure {
            Failure::Timeout => FailureLabel::Timeout,
            Failure::Unsupported => FailureLabel::Unsupported,
            Failure::Other { .. } => FailureLabel::Other,
        }
    }
}

impl From<FailureLabel> for Label {
    fn from(failure: FailureLabel) -> Self {
        match failure {
            FailureLabel::Timeout => Label::new("failure", "timeout"),
            FailureLabel::Unsupported => Label::new("failure", "unsupported"),
            FailureLabel::Other => Label::new("failure", "other"),
        }
    }
}
