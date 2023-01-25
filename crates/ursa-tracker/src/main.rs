use axum::http::StatusCode;
use axum::{
    extract::{ConnectInfo, Extension},
    routing::{get, post},
    Json, Router,
};
use hyper::HeaderMap;
use rocksdb::{IteratorMode, WriteBatch, DB};
use serde_json::{json, Value};
use std::{
    env,
    net::SocketAddr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::{
    ip_api::get_ip_info,
    types::{Node, PrometheusDiscoveryChunk, TrackerRegistration},
};

mod ip_api;
mod types;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::new(
            env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db = Arc::new(DB::open_default("tracker_db").unwrap());
    let token = env::var("IPINFO_TOKEN").expect("IPINFO_TOKEN is not set");

    let app = Router::new()
        .route("/register", post(registration_handler))
        .route("/http_sd", get(http_sd_handler))
        .layer(Extension(db))
        .layer(Extension(token));

    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
    info!("Ursa tracker listening on {addr}");

    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

/// Track a new peer registration in the database
async fn registration_handler(
    headers: HeaderMap,
    ConnectInfo(req_addr): ConnectInfo<SocketAddr>,
    db: Extension<Arc<DB>>,
    token: Extension<String>,
    Json(registration): Json<TrackerRegistration>,
) -> (StatusCode, Json<Value>) {
    let id = registration.id;
    info!("Received registration for: {id}");

    // todo: registration verification

    let addr = registration.addr.clone().unwrap_or_else(|| {
        // if no dns or ip address is provided, use the address of the request.
        // Prefer X-Forwarded-For header if present from reverse proxy. otherwise, use the
        // address of the request
        let ip = headers
            .get("X-Forwarded-For")
            .map(|x| x.to_str().unwrap().to_string())
            .unwrap_or_else(|| req_addr.ip().to_string());
        (ip != "127.0.0.1").then_some(ip).unwrap_or_default()
    });

    let info = match get_ip_info(token.0.clone(), addr.clone()).await {
        Ok(ip_info) => ip_info,
        Err(e) => {
            info!("Failed to lookup ip {}: {}", addr, e);
            return (StatusCode::SERVICE_UNAVAILABLE, Json(json!(e.to_string())));
        }
    };

    let entry = Node::from_info(
        &registration,
        info.addr,
        info.geo,
        info.timezone,
        info.country,
    );
    let json = json!(entry);

    info!("Storing node {id} with config {entry:?}");

    let mut batch = WriteBatch::default();
    batch.put(id.to_base58().as_bytes(), json.to_string().as_bytes());
    match db.write(batch) {
        Ok(_) => (StatusCode::OK, Json(json)),
        Err(e) => {
            error!("Error writing to db: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!(e.to_string())),
            )
        }
    }
}

/// Prometheus HTTP Service Discovery
async fn http_sd_handler(db: Extension<Arc<DB>>) -> (StatusCode, Json<Value>) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let services: Vec<PrometheusDiscoveryChunk> = db
        .iterator(IteratorMode::Start)
        .filter_map(|res| {
            if let Ok((_, v)) = res {
                let node: Node = serde_json::from_slice(&v).unwrap();
                // only return registrations in the last 1 month
                if now - node.last_registered < 2629800000 {
                    Some(node.into())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    (StatusCode::OK, Json(json!(services)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::PeerId;

    static INIT: std::sync::Once = std::sync::Once::new();

    fn tracer() {
        INIT.call_once(|| {
            tracing_subscriber::registry()
                .with(EnvFilter::new("info"))
                .with(tracing_subscriber::fmt::layer())
                .init();
        });
    }

    fn db() -> Arc<DB> {
        Arc::new(DB::open_default("tracker_db").unwrap())
    }

    async fn make_registration(
        db: Arc<DB>,
        addr: Option<String>,
        id: PeerId,
    ) -> (StatusCode, Json<Value>) {
        let data = TrackerRegistration {
            id,
            addr,
            p2p_port: Some(6009),
            http_port: Some(4069),
            agent: "".to_string(),
            telemetry: None,
        };
        registration_handler(
            HeaderMap::new(),
            ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 6969))),
            Extension(db),
            Extension(env::var("IPINFO_TOKEN").expect("IPINFO_TOKEN is not set")),
            Json(data),
        )
        .await
    }

    #[ignore]
    #[tokio::test]
    async fn local_node_registration() {
        tracer();
        let db = db();
        let id = PeerId::random();

        let res = make_registration(db.clone(), None, id).await;
        info!("{:?}", res);
        assert_eq!(res.0, 200);

        db.delete(id.to_string().as_bytes()).unwrap();
    }

    #[ignore]
    #[tokio::test]
    async fn remote_node_registration() {
        tracer();
        let db = db();
        let id = PeerId::random();

        let res = make_registration(db.clone(), Some("8.8.8.8".to_string()), id).await;
        info!("{:?}", res);
        assert_eq!(res.0, 200);
        db.delete(id.to_string().as_bytes()).unwrap();
    }

    #[ignore]
    #[tokio::test]
    async fn dns_node_registration() {
        tracer();
        let db = db();
        let id = PeerId::random();

        let res = make_registration(db.clone(), Some("google.com".into()), id).await;
        info!("{:?}", res.1.to_string());
        assert_eq!(res.0, 200);

        db.delete(id.to_string().as_bytes()).unwrap();
    }

    #[ignore]
    #[tokio::test]
    async fn prometheus_http_sd() {
        tracer();
        let db = db();
        let id = PeerId::random();

        let res = make_registration(db.clone(), None, id).await;
        info!("{:?}: {}", res.0, res.1.to_string());
        assert_eq!(res.0, 200);

        let res = http_sd_handler(Extension(db.clone())).await;
        info!("{:?}: {}", res.0, res.1.to_string());
        assert_eq!(res.0, 200);
        if let Value::Array(services) = res.1 .0 {
            assert_eq!(services.len(), 1);
        } else {
            panic!("Expected array");
        }

        db.delete(id.to_string().as_bytes()).unwrap();
    }
}
