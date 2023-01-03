mod model;
mod route;

use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use axum::{
    extract::{DefaultBodyLimit, Extension},
    routing::get,
    Router,
};
use axum_server::{tls_rustls::RustlsConfig, Handle};
use bytes::Bytes;
use route::api::v1::get::get_car_handler;
use tokio::{
    select, spawn,
    sync::{broadcast::Receiver, RwLock},
};
use tower::limit::concurrency::ConcurrencyLimitLayer;
use tower_http::{
    compression::CompressionLayer,
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnEos, DefaultOnFailure, DefaultOnResponse, TraceLayer},
};
use tracing::{debug, info, Level};

use crate::{
    config::{GatewayConfig, ServerConfig},
    worker::cache::ServerCache,
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
                request_body_limit,
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

    let app = Router::new()
        .route("/ping", get(|| async { "pong" }))
        .route("/:cid", get(get_car_handler::<Cache>))
        .layer(Extension(config))
        .layer(Extension(cache))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().include_headers(true))
                .on_body_chunk(|chunk: &Bytes, latency: Duration, _: &tracing::Span| {
                    debug!(size_bytes = chunk.len(), latency = ?latency, "sending body chunk")
                })
                .on_eos(DefaultOnEos::new().level(Level::INFO))
                .on_failure(DefaultOnFailure::new().level(Level::ERROR))
                .on_response(DefaultOnResponse::new().level(Level::INFO).include_headers(true)),
        )
        .layer(CompressionLayer::new())
        .layer(TimeoutLayer::new(Duration::from_millis(*request_timeout)))
        .layer(DefaultBodyLimit::max(*request_body_limit as usize))
        .layer(ConcurrencyLimitLayer::new(*concurrency_limit as usize));

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
                info!("Server remains alive connections: {}", handle.connection_count());
            }
        }
    }
}
