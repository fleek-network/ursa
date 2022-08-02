use anyhow::Result;
use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::IntoResponse,
    routing::get,
    Router,
};
// use libp2p_metrics::{Metrics, Recorder};
// use libp2p_swarm::{
//     SwarmEvent,
// };
// use prometheus_client::{metrics::info::Info, registry::Registry};
use metrics::{
    Gauge,register_gauge, increment_gauge, decrement_gauge,
    Counter, register_counter, increment_counter,
    // Histogram, register_histogram, histogram,
};
use std::net::SocketAddr;
use tracing::{info};

use crate::{
    events,
    metrics::MetricsRecorder,
    config::MetricsServiceConfig,
};

pub const ACTIVE_CONNECTED_PEERS: &str = "active_connected_peers";
pub const REQUEST_RECEIVED: &str = "requests_receiveds";

#[derive(Clone)]
pub struct MetricsService {
    active_connected_peers: Gauge,
    rpc_request_received: Counter,
}
impl MetricsService {
    pub fn new() -> Self {
        Self {
            active_connected_peers: register_gauge!(ACTIVE_CONNECTED_PEERS),
            rpc_request_received: register_counter!(REQUEST_RECEIVED),
        }
    }

    pub async fn start(&self, conf: &MetricsServiceConfig) -> Result<()> {
        let router = Router::new()
            .route("/ping", get(get_ping_handler))
            .route(conf.api_path.as_str(), get(get_metrics_handler));

        let http_address = SocketAddr::from(([0, 0, 0, 0], conf.port.parse::<u16>().unwrap()));
        info!("listening on {}", http_address);
        axum::Server::bind(&http_address)
            .serve(router.into_make_service())
            .await?;

        Ok(())
    }

    pub async fn track_request<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {

    }
}

impl MetricsRecorder for MetricsService {
    fn record(&self, event_name: &str) {
        info!("capturing event {:?}", event_name);
        match event_name {
            events::PEER_CONNECTED => {
                increment_gauge!(ACTIVE_CONNECTED_PEERS, 1.0);
            }
            events::PEER_DISCONNECTED => {
                decrement_gauge!(ACTIVE_CONNECTED_PEERS, 1.0);
            }

            events::RPC_REQUEST_RECEIVED => {
                increment_counter!(REQUEST_RECEIVED);
            }
            _ => info!("event name {:?} no match found", event_name)
        }
    }
}


pub async fn get_ping_handler() -> (StatusCode, String) {
    (StatusCode::OK, "pong".to_string())
}

pub async fn get_metrics_handler() -> (StatusCode, String) {
    (StatusCode::OK, "/metrics handler".to_string())
}


// mod tests {
//     use async_std::task;
//     use crate::{events, service::MetricsService, metrics::MetricsRecorder};

//     #[test]
//     fn test_active_connected_peers() {
//         let metrics_svc = MetricsService::new();

//         async fn capture_events(msvc: MetricsService) {
//             for _ in 0..10 {
//                 msvc.record(events::PEER_CONNECTED);
//             }
//         }

//         task::spawn(capture_events(metrics_svc.clone()));
//     }
// }