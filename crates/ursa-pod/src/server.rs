use bytes::BytesMut;
use futures::SinkExt;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tracing::error;

use crate::{
    codec::{Reason, UrsaCodec, UrsaCodecError, UrsaFrame},
    types::{Blake3Cid, BlsSignature, Secp256k1AffinePoint, Secp256k1PublicKey},
};

const IO_CHUNK_SIZE: usize = 16 * 1024;

/// Backend trait used by [`UfdpServer`] to access external data
pub trait Backend: Send + Sync + 'static {
    /// Get the raw content of a block.
    fn raw_block(&self, cid: &Blake3Cid, block: u64) -> Option<&[u8]>;

    /// Get a decryption_key for a block, includes a block request id
    fn decryption_key(&self, request_id: u64) -> (Secp256k1AffinePoint, u64);

    /// Get a clients current balance.
    fn get_balance(&self, pubkey: Secp256k1PublicKey) -> u128;

    /// Save a batch of transactions to be submitted to consensus.
    fn save_batch(&self, batch: BlsSignature) -> Result<(), String>;
}

/// UFDP Server. Handles any stream of data supporting [`AsyncWrite`] + [`AsyncRead`]
pub struct UfdpServer<B: Backend> {
    backend: Arc<B>,
}

impl<B> UfdpServer<B>
where
    B: Backend,
{
    pub fn new(backend: B) -> Result<Self, UrsaCodecError> {
        Ok(Self {
            backend: Arc::new(backend),
        })
    }

    /// Handle a connection. Spawns a tokio task and begins the session loop
    pub fn handle<S: AsyncWrite + AsyncRead + Unpin + Send + 'static>(
        &self,
        stream: S,
    ) -> Result<(), UrsaCodecError> {
        let backend = self.backend.clone();
        tokio::spawn(UfdpConnection::new(stream, backend).serve());
        Ok(())
    }
}

struct UfdpConnection<S, B> {
    transport: Framed<S, UrsaCodec>,
    backend: Arc<B>,
}

impl<S: AsyncWrite + AsyncRead + Unpin, B: Backend> UfdpConnection<S, B> {
    #[inline(always)]
    pub fn new(stream: S, backend: Arc<B>) -> Self {
        Self {
            transport: Framed::new(stream, UrsaCodec::default()),
            backend,
        }
    }

    pub async fn serve(mut self) {
        // Step 1: Perform the handshake.
        self.handshake().await;

        // Step 2: Handle requests.
        while let Some(Ok(frame)) = self.transport.next().await {
            match frame {
                UrsaFrame::ContentRequest { hash } => {
                    self.deliver_content(hash).await;
                }
                f => {
                    error!("Terminating, unexpected frame: {f:?}");
                    self.transport
                        .feed(UrsaFrame::TerminationSignal(Reason::UnexpectedFrame))
                        .await
                        .expect("send termination signal");
                }
            }
        }
    }

    #[inline(always)]
    async fn handshake(&mut self) {
        match self.transport.next().await.expect("handshake request") {
            Ok(UrsaFrame::HandshakeRequest { lane, .. }) => {
                let lane = lane.unwrap_or({
                    // todo: lane management
                    0
                });

                // Use send here because we want to flush the handshake response immediately.
                self.transport
                    .send(UrsaFrame::HandshakeResponse {
                        pubkey: [2; 33],
                        epoch_nonce: 1000,
                        lane,
                        last: None,
                    })
                    .await
                    .expect("handshake response");
            }
            _ => return,
        }
    }

    #[inline(always)]
    async fn deliver_content(&mut self, cid: Blake3Cid) {
        let mut block_number = 0;
        while let Some(block) = self.backend.raw_block(&cid, block_number) {
            block_number += 1;

            let proof = BytesMut::from(b"dummy_proof".as_slice());
            let decryption_key = [0; 33];
            let proof_len = proof.len() as u64;
            let block_len = block.len() as u64;

            self.transport
                .send(UrsaFrame::ContentResponse {
                    compression: 0,
                    proof_len,
                    block_len,
                    signature: [1u8; 64],
                })
                .await
                .expect("send content response");

            // --- experiment: try sending the raw data as is using the underlying IO.

            {
                let io = self.transport.get_mut();

                let mut proof: &[u8] = proof.as_ref();
                while !proof.is_empty() {
                    let bytes = io.write(&proof).await.unwrap();
                    proof = &proof[bytes..];
                }

                let mut block: &[u8] = block;
                while !block.is_empty() {
                    let bytes = io.write(&block).await.unwrap();
                    block = &block[bytes..];
                }
            }

            // wait for delivery acknowledgment
            // match self.transport.next().await {
            //     Some(Ok(UrsaFrame::DecryptionKeyRequest { .. })) => {
            //         // todo: transaction manager (batch and store tx)
            //     }
            //     Some(Ok(f)) => error!("Unexpected frame {f:?}"),
            //     Some(Err(e)) => error!("Codec error: {e:?}"),
            //     None => error!("Connection closed"),
            // }

            // send decryption key
            // self.transport
            //     .send(UrsaFrame::DecryptionKeyResponse { decryption_key })
            //     .await
            //     .expect("send decryption key");
        }

        self.transport
            .send(UrsaFrame::EndOfRequestSignal)
            .await
            .expect("send EOR");
    }
}
