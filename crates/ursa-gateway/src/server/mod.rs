mod model;
mod route;

use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use axum::{
    extract::Extension,
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router, ServiceExt,
};
use axum_prometheus::PrometheusMetricLayerBuilder;
use axum_server::{tls_rustls::RustlsConfig, Handle};
use axum_tracing_opentelemetry::opentelemetry_tracing_layer;
use route::api::v1::get::get_car_handler;
use serde_json::json;
use tokio::{
    select, spawn,
    sync::{broadcast::Receiver, RwLock},
};
use tower::limit::concurrency::ConcurrencyLimitLayer;
use tower_http::{
    catch_panic::CatchPanicLayer,
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    normalize_path::NormalizePath,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnFailure, DefaultOnResponse, TraceLayer},
};
use tracing::{error, info, Level};

use crate::{
    config::{GatewayConfig, ServerConfig},
    server::model::HttpResponse,
    worker::cache::server::ServerCache,
};

pub async fn start<Cache: ServerCache>(
    config: Arc<RwLock<GatewayConfig>>,
    cache: Arc<RwLock<Cache>>,
    shutdown_rx: Receiver<()>,
) -> Result<()> {
    let config_reader = Arc::clone(&config);
    let GatewayConfig {
        server:
            ServerConfig {
                addr,
                port,
                cert_path,
                key_path,
                concurrency_limit,
                request_timeout,
                ..
            },
        ..
    } = &(*config_reader.read().await);

    let rustls_config = RustlsConfig::from_pem_file(&cert_path, &key_path)
        .await
        .with_context(|| {
            format!("Failed to init tls from: cert: {cert_path:?}: path: {key_path:?}")
        })?;

    let addr = SocketAddr::from((
        addr.parse::<Ipv4Addr>()
            .with_context(|| format!("Failed to parse IPv4 with: {addr}"))?,
        *port,
    ));

    let (prometheus_layer, metric_handle) = PrometheusMetricLayerBuilder::new()
        .with_ignore_patterns(&["/metrics", "/ping"])
        .with_default_metrics()
        .build_pair();

    let app = NormalizePath::trim_trailing_slash(
        Router::new()
            .route("/ping", get(|| async { "pong" }))
            .route("/:cid", get(get_car_handler::<Cache>))
            .route("/metrics", get(|| async move { metric_handle.render() }))
            .layer(Extension(config))
            .layer(Extension(cache))
            .layer(CatchPanicLayer::custom(recover))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().include_headers(true))
                    .on_failure(DefaultOnFailure::new().level(Level::ERROR))
                    .on_response(
                        DefaultOnResponse::new()
                            .level(Level::INFO)
                            .include_headers(true)
                            .latency_unit(tower_http::LatencyUnit::Micros),
                    ),
            )
            .layer(PropagateRequestIdLayer::x_request_id())
            .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid {}))
            .layer(opentelemetry_tracing_layer())
            .layer(
                CorsLayer::new()
                    .allow_methods([Method::GET])
                    .allow_origin(Any),
            )
            .layer(CompressionLayer::new())
            .layer(TimeoutLayer::new(Duration::from_millis(*request_timeout)))
            .layer(prometheus_layer)
            .layer(ConcurrencyLimitLayer::new(*concurrency_limit as usize)),
    );

    info!("Server listening on {addr}");

    let handle = Handle::new();
    spawn(graceful_shutdown(handle.clone(), shutdown_rx));

    axum_server::bind_rustls(addr, rustls_config)
        .handle(handle)
        .serve(app.into_make_service())
        .await
        .context("Failed to start server")?;

    Ok(())
}

async fn graceful_shutdown(handle: Handle, mut shutdown_rx: Receiver<()>) {
    select! {
        _ = shutdown_rx.recv() => {
            handle.graceful_shutdown(Some(Duration::from_secs(30)));
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                info!("Server maintains alive connections: {}", handle.connection_count());
            }
        }
    }
}

fn recover(e: Box<dyn std::any::Any + Send + 'static>) -> Response {
    let e = if let Some(e) = e.downcast_ref::<String>() {
        e.to_string()
    } else if let Some(e) = e.downcast_ref::<&str>() {
        e.to_string()
    } else {
        "Unknown panic message".to_string()
    };
    error!("Unhandled error: {e:?}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!(HttpResponse {
            message: Some("Internal Server Error".into()),
        })),
    )
        .into_response()
}
