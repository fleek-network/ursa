use libp2p::request_response::{RequestResponseEvent, RequestResponseMessage};
use metrics::{increment_counter, Label};
use std::fmt::Debug;

impl<TRequest: Debug, TResponse: Debug> super::Recorder
    for RequestResponseEvent<TRequest, TResponse>
{
    fn record(&self) {
        match self {
            RequestResponseEvent::Message { message, peer, .. } => {
                match message {
                    RequestResponseMessage::Request {
                        request_id,
                        request,
                        ..
                    } => {
                        increment_counter!("req-res_total_request_received");
                        increment_counter!(
                            "req-res_request_received",
                            vec![
                                Label::new("peer", peer.to_string()),
                                Label::new("request_id", request_id.to_string()),
                                Label::new("request", format!("{request:?}")),
                            ]
                        );
                    }
                    RequestResponseMessage::Response { request_id, .. } => {
                        increment_counter!("req-res_total_response_sent");
                        increment_counter!(
                            "req-res_response_sent",
                            vec![
                                Label::new("peer", peer.to_string()),
                                Label::new("request_id", request_id.to_string()),
                                // channel?
                            ]
                        );
                    }
                }
            }
            RequestResponseEvent::OutboundFailure { .. } => {}
            RequestResponseEvent::InboundFailure { .. } => {}
            RequestResponseEvent::ResponseSent { .. } => {}
        }
    }
}
