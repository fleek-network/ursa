use anyhow::{anyhow, Result};
use async_abci::codec::{decode_length_delimited, encode_length_delimited};
use bytes::{Buf, BytesMut};
use tm_protos::abci::{
    request, response, Request, RequestApplySnapshotChunk, RequestBeginBlock, RequestCheckTx,
    RequestCommit, RequestDeliverTx, RequestEcho, RequestEndBlock, RequestFlush, RequestInfo,
    RequestInitChain, RequestListSnapshots, RequestLoadSnapshotChunk, RequestOfferSnapshot,
    RequestQuery, Response, ResponseApplySnapshotChunk, ResponseBeginBlock, ResponseCheckTx,
    ResponseCommit, ResponseDeliverTx, ResponseEcho, ResponseEndBlock, ResponseFlush, ResponseInfo,
    ResponseInitChain, ResponseListSnapshots, ResponseLoadSnapshotChunk, ResponseOfferSnapshot,
    ResponseQuery,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

/// The size of the read buffer for the client in its receiving of responses
/// from the server.
pub const DEFAULT_CLIENT_READ_BUF_SIZE: usize = 1024;

pub struct ClientBuilder {
    read_buf_size: usize,
}

impl ClientBuilder {
    /// Builder constructor.
    pub fn new(read_buf_size: usize) -> Self {
        Self { read_buf_size }
    }

    /// Client constructor that attempts to connect to the given network
    /// address.
    pub async fn connect<P: AsRef<std::path::Path>>(self, addr: P) -> Result<Client> {
        let stream = UnixStream::connect(addr).await?;
        Ok(Client {
            codec: ClientCodec::new(stream, self.read_buf_size),
        })
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            read_buf_size: DEFAULT_CLIENT_READ_BUF_SIZE,
        }
    }
}

pub struct Client {
    codec: ClientCodec,
}

macro_rules! perform {
    ($self:expr, $type:ident, $req:expr) => {
        match $self.perform(request::Value::$type($req)).await? {
            response::Value::$type(r) => Ok(r),
            r => Err(anyhow!(
                "unexpected server response type: expected {0}, but got {1:?}",
                stringify!($type).to_string(),
                r
            )
            .into()),
        }
    };
}

impl Client {
    /// Ask the ABCI server to echo back a message.
    pub async fn echo(&mut self, req: RequestEcho) -> Result<ResponseEcho> {
        perform!(self, Echo, req)
    }

    /// Request information about the ABCI application.
    pub async fn info(&mut self, req: RequestInfo) -> Result<ResponseInfo> {
        perform!(self, Info, req)
    }

    /// To be called once upon genesis.
    pub async fn init_chain(&mut self, req: RequestInitChain) -> Result<ResponseInitChain> {
        //perform!(self, InitChain, req)
        self.codec
            .send(Request {
                value: Some(request::Value::InitChain(req)),
            })
            .await?;
        self.codec
            .send(Request {
                value: Some(request::Value::Flush(RequestFlush {})),
            })
            .await?;

        let res = self
            .codec
            .next()
            .await
            .ok_or_else(|| anyhow!("Server connection terminated"))??;

        match res
            .value
            .ok_or_else(|| anyhow!("unexpected server response"))
        {
            Ok(response::Value::InitChain(r)) => Ok(r),
            _ => {
                tracing::error!("Wrong type");
                Ok(ResponseInitChain::default())
            }
        }
    }

    /// Query the application for data at the current or past height.
    pub async fn query(&mut self, req: RequestQuery) -> Result<ResponseQuery> {
        perform!(self, Query, req)
    }

    /// Check the given transaction before putting it into the local mempool.
    pub async fn check_tx(&mut self, req: RequestCheckTx) -> Result<ResponseCheckTx> {
        perform!(self, CheckTx, req)
    }

    /// Signal the beginning of a new block, prior to any `DeliverTx` calls.
    pub async fn begin_block(&mut self, req: RequestBeginBlock) -> Result<ResponseBeginBlock> {
        perform!(self, BeginBlock, req)
    }

    /// Apply a transaction to the application's state.
    pub async fn deliver_tx(&mut self, req: RequestDeliverTx) -> Result<ResponseDeliverTx> {
        perform!(self, DeliverTx, req)
    }

    /// Signal the end of a block.
    pub async fn end_block(&mut self, req: RequestEndBlock) -> Result<ResponseEndBlock> {
        perform!(self, EndBlock, req)
    }

    pub async fn flush(&mut self) -> Result<ResponseFlush> {
        perform!(self, Flush, RequestFlush {})
    }

    /// Commit the current state at the current height.
    pub async fn commit(&mut self) -> Result<ResponseCommit> {
        perform!(self, Commit, RequestCommit {})
    }

    /// Used during state sync to discover available snapshots on peers.
    pub async fn list_snapshots(&mut self) -> Result<ResponseListSnapshots> {
        perform!(self, ListSnapshots, RequestListSnapshots {})
    }

    /// Called when bootstrapping the node using state sync.
    pub async fn offer_snapshot(
        &mut self,
        req: RequestOfferSnapshot,
    ) -> Result<ResponseOfferSnapshot> {
        perform!(self, OfferSnapshot, req)
    }

    /// Used during state sync to retrieve chunks of snapshots from peers.
    pub async fn load_snapshot_chunk(
        &mut self,
        req: RequestLoadSnapshotChunk,
    ) -> Result<ResponseLoadSnapshotChunk> {
        perform!(self, LoadSnapshotChunk, req)
    }

    /// Apply the given snapshot chunk to the application's state.
    pub async fn apply_snapshot_chunk(
        &mut self,
        req: RequestApplySnapshotChunk,
    ) -> Result<ResponseApplySnapshotChunk> {
        perform!(self, ApplySnapshotChunk, req)
    }

    async fn perform(&mut self, req: request::Value) -> Result<response::Value> {
        self.codec.send(Request { value: Some(req) }).await?;
        let res = self
            .codec
            .next()
            .await
            .ok_or_else(|| anyhow!("Server connection terminated"))??;
        res.value
            .ok_or_else(|| anyhow!("unexpected server response"))
    }
}

pub struct ClientCodec {
    stream: UnixStream,
    // Long-running read buffer
    read_buf: BytesMut,
    // Fixed-length read window
    read_window: Vec<u8>,
    write_buf: BytesMut,
}

impl ClientCodec {
    /// Constructor.
    pub fn new(stream: UnixStream, read_buf_size: usize) -> Self {
        Self {
            stream,
            read_buf: BytesMut::new(),
            read_window: vec![0_u8; read_buf_size],
            write_buf: BytesMut::new(),
        }
    }

    pub async fn next(&mut self) -> Option<Result<Response>> {
        loop {
            // Try to decode an incoming message from our buffer first
            match decode_length_delimited::<Response>(&mut self.read_buf) {
                Ok(Some(incoming)) => return Some(Ok(incoming)),
                Err(e) => return Some(Err(anyhow!("failed decoding stream: {:?}", e))),
                _ => (), // not enough data to decode a message, let's continue.
            }

            // If we don't have enough data to decode a message, try to read
            // more
            let bytes_read = match self.stream.read(self.read_window.as_mut()).await {
                Ok(br) => br,
                Err(e) => return Some(Err(anyhow!("StdIoError: {:?}", e))),
            };
            if bytes_read == 0 {
                // The underlying stream terminated
                return None;
            }
            self.read_buf
                .extend_from_slice(&self.read_window[..bytes_read]);
        }
    }

    /// Send a message using this codec.
    pub async fn send(&mut self, message: Request) -> Result<()> {
        encode_length_delimited(message, &mut self.write_buf)
            .map_err(|e| anyhow!("Failed to encode message: {:?}", e))?;
        while !self.write_buf.is_empty() {
            let bytes_written = self
                .stream
                .write(self.write_buf.as_ref())
                .await
                .map_err(|e| anyhow!("StdIoError: {:?}", e))?;

            if bytes_written == 0 {
                return Err(anyhow!("failed to write to underlying stream"));
            }
            self.write_buf.advance(bytes_written);
        }

        self.stream
            .flush()
            .await
            .map_err(|e| anyhow!("StdIoError : {:?}", e))?;

        Ok(())
    }
}
