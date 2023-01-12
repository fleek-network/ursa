#[cfg(test)]
mod tests {
    use crate::{
        api::NodeNetworkInterface,
        server::Server,
        tests::{init, setup_logger},
    };
    use anyhow::Result;
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };

    use serde_json::{json, Value};
    use std::sync::Arc;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_http_server() -> Result<()> {
        setup_logger();
        let (ursa_service, provider_engine, store) = init()?;

        let interface = Arc::new(NodeNetworkInterface::new(
            Arc::clone(&store),
            ursa_service.command_sender(),
            provider_engine.command_sender(),
            Default::default(),
        ));
        let server = Server::new(interface);
        let metrics = ursa_metrics::routes::init();
        let http_app = server.http_app(provider_engine.router(), Some(metrics));

        let response = http_app
            .oneshot(Request::builder().uri("/ping").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(&body[..], b"pong");
        Ok(())
    }

    #[tokio::test]
    async fn test_rpc_server() -> Result<()> {
        setup_logger();
        let (ursa_service, provider_engine, store) = init()?;

        let interface = Arc::new(NodeNetworkInterface::new(
            Arc::clone(&store),
            ursa_service.command_sender(),
            provider_engine.command_sender(),
            Default::default(),
        ));
        let server = Server::new(interface);
        let rpc_app = server.rpc_app();

        let req = serde_json::to_vec(&json!({
            "jsonrpc": "2.0",
            "method":"ursa_listener_addresses",
            "params":[],
            "id":1,
        }))
        .unwrap();

        let response = rpc_app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/rpc/v0")
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(req))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let value: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            *value.get("result").unwrap(),
            json!(["/ip4/127.0.0.1/tcp/6009".to_string()])
        );
        Ok(())
    }
}
