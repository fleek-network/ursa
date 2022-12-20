use libp2p::relay::v2::relay::Event;
use metrics::increment_counter;

impl super::Recorder for Event {
    fn record(&self) {
        increment_counter!(match self {
            Event::ReservationReqAccepted { .. } => "relay_reservation_req_accepted",
            Event::ReservationReqAcceptFailed { .. } => "relay_reservation_req_accept_failed",
            Event::ReservationReqDenied { .. } => "relay_reservation_req_denied",
            Event::ReservationReqDenyFailed { .. } => "relay_reservation_req_deny_failed",
            Event::ReservationTimedOut { .. } => "relay_reservation_timed_out",
            Event::CircuitReqReceiveFailed { .. } => "relay_circuit_req_receive_failed",
            Event::CircuitReqDenied { .. } => "relay_circuit_req_denied",
            Event::CircuitReqDenyFailed { .. } => "relay_circuit_req_deny_failed",
            Event::CircuitReqAccepted { .. } => "relay_circuit_req_accepted",
            Event::CircuitReqOutboundConnectFailed { .. } =>
                "relay_circuit_req_outbound_connect_failed",
            Event::CircuitReqAcceptFailed { .. } => "relay_circuit_req_accept_failed",
            Event::CircuitClosed { .. } => "relay_circuit_closed",
        });
    }
}
