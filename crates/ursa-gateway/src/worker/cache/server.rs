use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use axum::{
    body::{HttpBody, StreamBody},
    http::{response::Parts, StatusCode},
};
use bytes::BufMut;
use tokio::{
    io::{duplex, AsyncWriteExt, DuplexStream},
    spawn,
    sync::{mpsc::UnboundedSender, oneshot},
};
use tokio_util::io::ReaderStream;
use tracing::{error, info_span, Instrument, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use super::{Cache, CacheCommand};
use crate::util::error::Error;

#[async_trait]
pub trait ServerCache: Send + Sync + 'static {
    async fn get_announce(
        &self,
        k: &str,
        no_cache: bool,
    ) -> Result<StreamBody<ReaderStream<DuplexStream>>, Error>;
}

#[async_trait]
impl ServerCache for Cache {
    async fn get_announce(
        &self,
        k: &str,
        no_cache: bool,
    ) -> Result<StreamBody<ReaderStream<DuplexStream>>, Error> {
        let (mut w, r) = duplex(self.stream_buf as usize);
        if no_cache {
            let span = info_span!("Fetch and insert with no cache");
            fetch_and_insert(k, &self.tx, w).instrument(span).await?;
        } else if let Some(data) = self.tlrfu.dirty_get(&String::from(k)) {
            let span = info_span!("Get announce with cache hit");
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
            spawn(
                async move {
                    let span = info_span!("Stream writing");
                    if let Err(e) = w.write_all(data.as_ref()).instrument(span).await {
                        error!("Failed to write to stream: {e:?}");
                    }
                }
                .instrument(span),
            );
        } else {
            let span = info_span!("Fetch and insert with cache missed");
            fetch_and_insert(k, &self.tx, w).instrument(span).await?;
        }
        Ok(StreamBody::new(ReaderStream::new(r)))
    }
}

async fn fetch_and_insert(
    k: &str,
    cmd_sender: &UnboundedSender<CacheCommand>,
    mut stream_writer: DuplexStream,
) -> Result<(), Error> {
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
    let mut body = match rx
        .await
        .map_err(|e| {
            error!("Failed to receive response from resolver: {e:?}");
            anyhow!("Failed to receive response from resolver")
        })??
        .into_parts()
    {
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
    let key = String::from(k); // move to [worker|writer] thread
    let tx = cmd_sender.clone(); // move to [worker|writer] thread
    spawn(
        async move {
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
        }
        .instrument(info_span!("Stream writing")),
    );
    Ok(())
}
