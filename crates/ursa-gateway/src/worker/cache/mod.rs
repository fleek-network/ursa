pub mod admin;
pub mod server;
pub mod worker;

use std::sync::Arc;

use anyhow::Result;
use axum::{body::Body, response::Response};
use bytes::Bytes;
use tokio::sync::{mpsc::UnboundedSender, oneshot};

use crate::{
    cache::{ByteSize, Tlrfu},
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
}

impl Cache {
    pub fn new(
        max_size: u64,
        ttl_buf: u128,
        tx: UnboundedSender<CacheCommand>,
        stream_buf: u64,
    ) -> Self {
        Self {
            tlrfu: Tlrfu::new(max_size, ttl_buf),
            tx,
            stream_buf,
        }
    }
}

#[derive(Debug)]
pub enum CacheCommand {
    GetSync {
        key: String,
    },
    InsertSync {
        key: String,
        value: Arc<Bytes>,
    },
    Fetch {
        cid: String,
        sender: oneshot::Sender<Result<Response<Body>, Error>>,
    },
    TtlCleanUp,
}
