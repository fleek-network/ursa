use crate::{
    ip_api::get_ip_info,
    types::{Node, NodeAnnouncement, PrometheusDiscoveryChunk},
};
use axum::{
    extract::{ConnectInfo, Extension},
    routing::{get, post},
    Json, Router,
};
use axum::http::StatusCode;
use serde_json::{json, Value};
use std::{env, net::SocketAddr, sync::Arc};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use sled::Db;

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

    let db = Arc::new(sled::open("db").unwrap());
    let token = env::var("IPINFO_TOKEN").expect("IPINFO_TOKEN is not set");

    let app = Router::new()
        .route("/announce", post(announcement_handler))
        .route("/http_sd", get(http_sd_handler))
        .layer(Extension(db))
        .layer(Extension(token));

    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
    println!("Ursa-tracker listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

/// Track a new peer announcement in the database
async fn announcement_handler(
    ConnectInfo(req_addr): ConnectInfo<SocketAddr>,
    db: Extension<Arc<Db>>,
    token: Extension<String>,
    Json(announcement): Json<NodeAnnouncement>,
) -> (StatusCode, Json<Value>) {
    let id = announcement.id;
    info!("Received announcement for: {}", id);

    // todo: announcement verification

    let addr = announcement.addr.clone().unwrap_or_else(|| {
        let ip = req_addr.ip().to_string();
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
        &announcement,
        info.ip,
        info.geo,
        info.timezone,
        info.country,
    );
    let json = json!(entry);

    info!("Storing node {} with config {:?}", id, entry);

    match db.0.insert(id.to_base58().as_bytes(), json.to_string().as_bytes()) {
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
async fn http_sd_handler(db: Extension<Arc<Db>>) -> (StatusCode, Json<Value>) {
    let services: Vec<PrometheusDiscoveryChunk> = db
        .0.iter()
        .filter_map(|i| {
            if let Ok((_, v)) = i {
                let node: Node = serde_json::from_slice(&v.as_ref()).unwrap();
                if !node.telemetry {
                    return None;
                }
                Some(node.into())
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

    async fn make_announcement(
        db: Arc<DB>,
        addr: Option<String>,
        id: PeerId,
    ) -> (StatusCode, Json<Value>) {
        let data = NodeAnnouncement {
            id,
            addr,
            storage: 0,
            p2p_port: Some(6009),
            telemetry: Some(true),
            metrics_port: Some(6009),
        };
        announcement_handler(
            ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 6969))),
            Extension(db),
            Extension(env::var("IPINFO_TOKEN").expect("IPINFO_TOKEN is not set")),
            Json(data),
        )
        .await
    }

    #[tokio::test]
    async fn local_node_announcement() {
        tracer();
        let db = db();
        let id = PeerId::random();

        let res = make_announcement(db.clone(), None, id).await;
        info!("{:?}", res);
        assert_eq!(res.0, 200);

        db.delete(id.to_string().as_bytes()).unwrap()
    }

    #[tokio::test]
    async fn remote_node_announcement() {
        tracer();
        let db = db();
        let id = PeerId::random();

        let res = make_announcement(db.clone(), Some("8.8.8.8".to_string()), id).await;
        info!("{:?}", res);
        assert_eq!(res.0, 200);

        db.delete(id.to_string().as_bytes()).unwrap()
    }

    // doesn't work with ipinfo
    // #[tokio::test]
    // async fn dns_node_announcement() {
    //     tracer();
    //     let db = db();
    //     let id = PeerId::random();
    //
    //     let res = make_announcement(db.clone(), None, id).await;
    //     info!("{:?}", res.1.to_string());
    //     assert_eq!(res.0, 200);
    //
    //     db.delete(id.to_string().as_bytes()).unwrap();
    // }

    #[tokio::test]
    async fn prometheus_http_sd() {
        tracer();
        let db = db();
        let id = PeerId::random();

        let res = make_announcement(db.clone(), None, id).await;
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
