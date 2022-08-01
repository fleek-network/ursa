use anyhow::Result;
use axum::{http::StatusCode, routing::get, Router};
// use libp2p_metrics::{Metrics, Recorder};
// use libp2p_swarm::{
//     SwarmEvent,
// };
// use prometheus_client::{metrics::info::Info, registry::Registry};
use metrics::{Gauge,register_gauge, increment_gauge, decrement_gauge};
use std::net::SocketAddr;
use tracing::{info};

use crate::{
    events,
    metrics::MetricsRecorder,
    config::MetricsServiceConfig,
};

pub const ACTIVE_CONNECTED_PEERS: &str = "active_connected_peers";

#[derive(Clone)]
pub struct MetricsService {
    active_connected_peers: Gauge,
}
impl MetricsService {
    pub fn new() -> Self {
        Self {
            active_connected_peers: register_gauge!(ACTIVE_CONNECTED_PEERS),
        }
    }

    // pub fn new() -> Self {
    //     let mut metric_registry = Registry::default();
    //     let metrics = Metrics::new(&mut metric_registry);
    //     let build_info = Info::new(vec![("version".to_string(), env!("CARGO_PKG_VERSION"))]);

    //     metric_registry.register(
    //         "build",
    //         "A sample metric with a constant value labeled by version",
    //         Box::new(build_info),
    //     );

    //     Self {
    //         metrics: metrics,
    //         registry: metric_registry,
    //     }
    // }

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
}

impl MetricsRecorder for MetricsService {
    fn record(&self, event_name: &str) {
        match event_name {
            events::PEER_CONNECTED => {
                info!("event {:?} captured", event_name);
                increment_gauge!(ACTIVE_CONNECTED_PEERS, 1.0);
            }
            events::PEER_DISCONNECTED => {
                info!("event {:?} captured", event_name);
                decrement_gauge!(ACTIVE_CONNECTED_PEERS, 1.0);
            }
            _ => info!("event name {:?} not match found", event_name)
        }
    }
}


pub async fn get_ping_handler() -> (StatusCode, String) {
    (StatusCode::OK, "pong".to_string())
}

pub async fn get_metrics_handler() -> (StatusCode, String) {
    (StatusCode::OK, "/metrics handler".to_string())
}


mod tests {
    use async_std::task;
    use crate::{events, service::MetricsService, metrics::MetricsRecorder};

    #[test]
    fn test_active_connected_peers() {
        let metrics_svc = MetricsService::new();

        async fn capture_events(msvc: MetricsService) {
            for _ in 0..10 {
                msvc.record(events::PEER_CONNECTED);
            }
        }
        task::spawn(capture_events(metrics_svc.clone()));
    }
}