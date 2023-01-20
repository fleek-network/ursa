pub mod admin;
pub mod server;
pub mod worker;

use std::sync::Arc;

use anyhow::Result;
use bytes::Bytes;
use opentelemetry::Context;
use tokio::sync::{mpsc::UnboundedSender, oneshot};

use crate::{
    cache::{ByteSize, Tlrfu},
    resolver::NodeResponse,
    util::error::Error,
};

impl ByteSize for Bytes {
    fn len(&self) -> usize {
        self.len()
    }
}

pub struct Cache {
    tlrfu: Tlrfu<Bytes>,
    tx: UnboundedSender<CacheCommand>,
    stream_buf: u64,
    cache_control_max_size: u64,
}

impl Cache {
    pub fn new(
        max_size: u64,
        ttl_buf: u128,
        tx: UnboundedSender<CacheCommand>,
        stream_buf: u64,
        cache_control_max_size: u64,
    ) -> Self {
        Self {
            tlrfu: Tlrfu::new(max_size, ttl_buf),
            tx,
            stream_buf,
            cache_control_max_size,
        }
    }
}

#[derive(Debug)]
pub enum CacheCommand {
    GetSync {
        key: String,
        ctx: Context,
    },
    InsertSync {
        key: String,
        value: Arc<Bytes>,
        ctx: Context,
    },
    Fetch {
        cid: String,
        sender: oneshot::Sender<Result<NodeResponse, Error>>,
        ctx: Context,
    },
    TtlCleanUp,
}
