mod route;

use {
    crate::{
        config::{GatewayConfig, ServerConfig},
        worker::cache::AdminCache,
    },
    anyhow::{Context, Result},
    axum::{
        extract::Extension,
        routing::{get, post},
        Router,
    },
    axum_server::tls_rustls::RustlsConfig,
    route::api::v1::{get::get_config_handler, post::purge_cache_handler},
    std::{
        net::{Ipv4Addr, SocketAddr},
        sync::Arc,
    },
    tokio::sync::RwLock,
    tracing::info,
};

pub async fn start<Cache: AdminCache>(
    config: Arc<RwLock<GatewayConfig>>,
    cache: Arc<RwLock<Cache>>,
) -> Result<()> {
    let config_reader = Arc::clone(&config);
    let GatewayConfig {
        admin_server:
            ServerConfig {
                addr,
                port,
                cert_path,
                key_path,
            },
        ..
    } = &(*config_reader.read().await);

    let rustls_config = RustlsConfig::from_pem_file(&cert_path, &key_path)
        .await
        .with_context(|| {
            format!("Failed to init tls from: cert: {cert_path:?}: path:{key_path:?}")
        })?;

    let addr = SocketAddr::from((
        addr.parse::<Ipv4Addr>()
            .with_context(|| format!("Failed to parse IPv4 with: {addr}"))?,
        *port,
    ));

    let app = Router::new()
        .route("/config", get(get_config_handler))
        .route("/purge-cache", post(purge_cache_handler::<Cache>))
        .layer(Extension(config))
        .layer(Extension(cache));

    info!("Admin server listening on {addr}");

    axum_server::bind_rustls(addr, rustls_config)
        .serve(app.into_make_service())
        .await
        .context("Server stopped")?;

    Ok(())
}
