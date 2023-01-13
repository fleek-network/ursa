use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use axum::response::IntoResponse;
use axum::{
    body::HttpBody,
    http::{response::Parts, StatusCode},
};
use bytes::BufMut;
use hyper::Body;
use tokio::{
    io::{duplex, AsyncWriteExt, DuplexStream},
    spawn,
    sync::{mpsc::UnboundedSender, oneshot},
};
use tokio_util::io::ReaderStream;
use tracing::{error, info, info_span, Instrument, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use super::{Cache, CacheCommand};
use crate::util::error::Error;

#[async_trait]
pub trait ServerCache: Send + Sync + 'static {
    async fn get_announce(&self, k: &str, no_cache: bool) -> Result<StreamBody, Error>;
}

#[async_trait]
impl ServerCache for Cache {
    async fn get_announce(&self, k: &str, no_cache: bool) -> Result<StreamBody, Error> {
        if no_cache {
            let span = info_span!("Cache invalidate");
            fetch(k, &self.tx)
                .instrument(span)
                .await
                .map(|(body, _)| StreamBody::Direct(body))
        } else if let Some(data) = self.tlrfu.dirty_get(&String::from(k)) {
            let (mut w, r) = duplex(self.stream_buf as usize);
            let span = info_span!("Cache hit");
            let data = Arc::clone(data);
            self.tx
                .send(CacheCommand::GetSync {
                    key: String::from(k),
                    ctx: Span::current().context(),
                })
                .map_err(|e| {
                    error!("Failed to dispatch GetSync command: {e:?}");
                    anyhow!("Failed to dispatch GetSync command")
                })?;
            let stream_writer = async move {
                let span = info_span!("Stream writing");
                if let Err(e) = w.write_all(data.as_ref()).instrument(span).await {
                    error!("Failed to write to stream: {e:?}");
                }
            };
            spawn(stream_writer.instrument(span));
            Ok(StreamBody::Duplex(r))
        } else {
            let span = info_span!("Cache missed");
            let cache_miss = async {
                let (mut body, content_size) = fetch(k, &self.tx).await?;
                if content_size > self.cache_control_max_size {
                    info!("Content size is {content_size}...skipping cache");
                    return Ok(StreamBody::Direct(body));
                }
                let key = String::from(k); // move to [worker|writer] thread
                let tx = self.tx.clone(); // move to [worker|writer] thread
                let (mut stream_writer, stream_reader) = duplex(self.stream_buf as usize);
                let stream_writer = async move {
                    let mut bytes = Vec::with_capacity(body.size_hint().lower() as usize);
                    while let Some(buf) = body.data().await {
                        match buf {
                            Ok(buf) => {
                                if let Err(e) = stream_writer.write_all(buf.as_ref()).await {
                                    error!("Failed to write to stream for {e:?}");
                                    return;
                                };
                                bytes.put(buf);
                            }
                            Err(e) => {
                                error!("Failed to read stream for {e:?}");
                                return;
                            }
                        }
                    }
                    if let Err(e) = tx.send(CacheCommand::InsertSync {
                        key,
                        value: Arc::new(bytes.into()),
                        ctx: Span::current().context(),
                    }) {
                        error!("Failed to dispatch InsertSync command: {e:?}");
                    };
                };
                spawn(stream_writer.instrument(info_span!("Stream writing")));
                Ok(StreamBody::Duplex(stream_reader))
            };
            cache_miss.instrument(span).await
        }
    }
}

/// Returns stream of bytes and advertised size of the content.
async fn fetch(k: &str, cmd_sender: &UnboundedSender<CacheCommand>) -> Result<(Body, u64), Error> {
    let (tx, rx) = oneshot::channel();
    cmd_sender
        .send(CacheCommand::Fetch {
            cid: String::from(k),
            sender: tx,
            ctx: Span::current().context(),
        })
        .map_err(|e| {
            error!("Failed to dispatch Fetch command: {e:?}");
            anyhow!("Failed to dispatch Fetch command")
        })?;
    let response = rx.await.map_err(|e| {
        error!("Failed to receive response from resolver: {e:?}");
        anyhow!("Failed to receive response from resolver")
    })??;
    let body = match response.resp.into_parts() {
        (
            Parts {
                status: StatusCode::OK,
                ..
            },
            body,
        ) => body,
        (parts, body) => {
            error!("Error requested provider with parts: {parts:?} and body: {body:?}");
            return Err(Error::Upstream(
                parts.status,
                "Error requested provider".to_string(),
            ));
        }
    };
    Ok((body, response.size))
}

pub enum StreamBody {
    Direct(Body),
    Duplex(DuplexStream),
}

impl IntoResponse for StreamBody {
    fn into_response(self) -> axum::response::Response {
        match self {
            StreamBody::Direct(body) => axum::body::StreamBody::new(body).into_response(),
            StreamBody::Duplex(duplex_stream) => {
                axum::body::StreamBody::new(ReaderStream::new(duplex_stream)).into_response()
            }
        }
    }
}
