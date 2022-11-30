use crate::{
    ip_api::get_ip_info,
    types::{Node, NodeAnnouncement, PrometheusDiscoveryChunk},
};
use axum::{
    extract::{ConnectInfo, Extension},
    routing::{get, post},
    Json, Router,
};
// use axum_server::tls_rustls::RustlsConfig;
use axum::http::StatusCode;
use rocksdb::{IteratorMode, WriteBatch, DB};
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod ip_api;
mod types;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = Arc::new(DB::open_default("tracker_db").unwrap());

    let app = Router::new()
        .route("/announce", post(announcement_handler))
        .route("/http_sd", get(http_sd_handler))
        .layer(Extension(db));

    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
    println!("Ursa-tracker listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();

    // let config =
    //     RustlsConfig::from_pem_file("self_signed_certs/cert.pem", "self_signed_certs/key.pem")
    //         .await
    //         .unwrap();
    //     axum_server::bind_rustls(addr, config)
    //         .serve(app.into_make_service_with_connect_info::<SocketAddr>())
    //         .await
    //         .unwrap();
}

/// Track a new peer announcement in the database
async fn announcement_handler(
    ConnectInfo(req_addr): ConnectInfo<SocketAddr>,
    db: Extension<Arc<DB>>,
    Json(announcement): Json<NodeAnnouncement>,
) -> (StatusCode, Json<Value>) {
    let id = announcement.id;
    info!("Received announcement for: {}", id);

    // todo: verify announcement.
    //       - check if the p2p port is reachable and id is valid
    //       - if telemetry is enabled, check if the metrics port is reachable and valid

    let addr = announcement
        .addr
        .clone()
        .unwrap_or_else(|| req_addr.ip().to_string());

    // lookup ourselves if the ip is localhost
    let addr = (addr.to_string() != "127.0.0.1").then_some(addr);

    let ip_info = match get_ip_info(addr).await {
        Ok(info) => info,
        Err(e) => {
            tracing::error!("ip-api error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!(e.to_string())),
            );
        }
    };

    let entry = Node::from_info(
        &announcement,
        ip_info.query,
        ip_info.geo,
        ip_info.timezone,
        ip_info.country_code,
    );
    let json = json!(entry);

    info!("Storing node {} with config {:?}", id, entry);

    let mut batch = WriteBatch::default();
    batch.put(id.to_base58().as_bytes(), json.to_string().as_bytes());
    match db.write(batch) {
        Ok(_) => (StatusCode::OK, Json(json)),
        Err(e) => {
            tracing::error!("Error writing to db: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!(e.to_string())),
            )
        }
    }
}

/// Prometheus HTTP Service Discovery
async fn http_sd_handler(db: Extension<Arc<DB>>) -> (StatusCode, Json<Value>) {
    let services: Vec<PrometheusDiscoveryChunk> = db
        .iterator(IteratorMode::Start)
        .filter_map(|(_, v)| {
            let node: Node = serde_json::from_slice(&v).unwrap();
            if !node.telemetry {
                return None;
            }
            Some(node.into())
        })
        .collect();

    (StatusCode::OK, Json(json!(services)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity::Keypair;
    use rocksdb::Options;

    fn tracer() {
        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::new(
                std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
            ))
            .init();
    }

    fn db() -> Arc<DB> {
        let mut opts = Options::default();
        opts.increase_parallelism(10);
        opts.create_if_missing(true);
        Arc::new(DB::open(&opts, "tracker_db").unwrap())
    }

    fn new_announcement() -> NodeAnnouncement {
        NodeAnnouncement {
            id: Keypair::generate_ed25519().public().into(),
            storage: 0,
            addr: None,
            p2p_port: Some(6009),
            telemetry: Some(true),
            metrics_port: Some(6009),
        }
    }

    #[tokio::test]
    async fn local_node_announcement() {
        tracer();
        let (announcement, db) = (new_announcement(), db());

        let res = announcement_handler(
            ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 4000))),
            Extension(db.clone()),
            Json(announcement.clone()),
        )
        .await;
        info!("{:?}", res);
        assert_eq!(res.0, 200);

        db.delete(announcement.id.to_string().as_bytes()).unwrap()
    }

    #[tokio::test]
    async fn remote_node_announcement() {
        tracer();
        let (announcement, db) = (new_announcement(), db());

        let res = announcement_handler(
            ConnectInfo(SocketAddr::from(([8, 8, 8, 8], 4000))),
            Extension(db.clone()),
            Json(announcement.clone()),
        )
        .await;
        info!("{:?}", res);
        assert_eq!(res.0, 200);

        db.delete(announcement.id.to_string().as_bytes()).unwrap()
    }

    #[tokio::test]
    async fn dns_node_announcement() {
        tracer();
        let (mut announcement, db) = (new_announcement(), db());
        announcement.addr = Some("google.com".to_string());

        let (status, res) = announcement_handler(
            ConnectInfo(SocketAddr::from(([8, 8, 8, 8], 4000))),
            Extension(db.clone()),
            Json(announcement.clone()),
        )
        .await;
        info!("{:?}", res.to_string());
        assert_eq!(status, 200);

        db.delete(announcement.id.to_string().as_bytes()).unwrap();
    }

    #[tokio::test]
    async fn prometheus_http_sd() {
        tracer();
        let (announcement, db) = (new_announcement(), db());

        let (status, res) = announcement_handler(
            ConnectInfo(SocketAddr::from(([8, 8, 8, 8], 4000))),
            Extension(db.clone()),
            Json(announcement.clone()),
        )
        .await;
        info!("{:?}: {}", status, res.to_string());
        assert_eq!(status, 200);

        let (status, res) = http_sd_handler(Extension(db.clone())).await;
        info!("{:?}: {}", status, res.clone().to_string());
        assert_eq!(status, 200);
        if let Value::Array(services) = res.0 {
            assert_eq!(services.len(), 1);
        } else {
            panic!("Expected array");
        }

        db.delete(announcement.id.to_string().as_bytes()).unwrap();
    }
}
