use std::sync::Arc;

use bytes::BytesMut;
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::error;

use crate::{
    connection::{
        consts::{CONTENT_REQ_TAG, HANDSHAKE_REQ_TAG},
        Reason, UfdpConnection, UrsaCodecError, UrsaFrame,
    },
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
pub struct UfdpHandler<S: AsyncRead + AsyncWrite + Unpin, B: Backend> {
    conn: UfdpConnection<S>,
    backend: Arc<B>,
}

impl<S: AsyncWrite + AsyncRead + Unpin, B: Backend> UfdpHandler<S, B> {
    #[inline(always)]
    pub fn new(stream: S, backend: B) -> Self {
        Self {
            conn: UfdpConnection::new(stream),
            backend: Arc::new(backend),
        }
    }

    pub async fn serve(mut self) -> Result<(), UrsaCodecError> {
        // Step 1: Perform the handshake.
        self.handshake().await?;

        // Step 2: Handle requests.
        while let Some(frame) = self.conn.read_frame(Some(CONTENT_REQ_TAG)).await? {
            match frame {
                UrsaFrame::ContentRequest { hash } => {
                    self.deliver_content(hash).await?;
                }
                f => {
                    error!("Terminating, unexpected frame: {f:?}");
                    self.conn
                        .termination_signal(Some(Reason::UnexpectedFrame))
                        .await?;
                    return Err(UrsaCodecError::UnexpectedFrame(f.tag().unwrap()));
                }
            }
        }

        Ok(())
    }

    #[inline(always)]
    async fn handshake(&mut self) -> Result<(), UrsaCodecError> {
        match self.conn.read_frame(Some(HANDSHAKE_REQ_TAG)).await? {
            Some(UrsaFrame::HandshakeRequest { lane, .. }) => {
                // send res frame
                self.conn
                    .write_frame(UrsaFrame::HandshakeResponse {
                        pubkey: [2; 33],
                        epoch_nonce: 1000,
                        lane: lane.unwrap_or(0),
                        last: None,
                    })
                    .await?;

                Ok(())
            }
            Some(f) => {
                error!("Terminating, unexpected frame: {f:?}");
                self.conn
                    .termination_signal(Some(Reason::UnexpectedFrame))
                    .await?;
                Err(UrsaCodecError::UnexpectedFrame(f.tag().unwrap()))
            }
            None => Err(UrsaCodecError::Unknown),
        }
    }

    #[inline(always)]
    async fn deliver_content(&mut self, cid: Blake3Cid) -> Result<(), UrsaCodecError> {
        let mut block_number = 0;
        while let Some(block) = self.backend.raw_block(&cid, block_number) {
            block_number += 1;

            let proof = BytesMut::from(b"dummy_proof".as_slice());
            let decryption_key = [0; 33];
            let proof_len = proof.len() as u64;
            let block_len = block.len() as u64;

            self.conn
                .write_frame(UrsaFrame::ContentResponse {
                    compression: 0,
                    proof_len,
                    block_len,
                    signature: [1u8; 64],
                })
                .await?;

            self.conn.write_frame(UrsaFrame::Buffer(proof)).await?;
            self.conn
                .write_frame(UrsaFrame::Buffer(block.into()))
                .await?;

            // wait for delivery acknowledgment
            match self.conn.read_frame(None).await? {
                Some(UrsaFrame::DecryptionKeyRequest { .. }) => {
                    // todo: transaction manager (batch and store tx)
                }
                Some(f) => {
                    error!("Terminating asdf, unexpected frame: {f:?}");
                    self.conn
                        .termination_signal(Some(Reason::UnexpectedFrame))
                        .await?;
                    return Err(UrsaCodecError::UnexpectedFrame(f.tag().unwrap()));
                }
                None => return Err(UrsaCodecError::Unknown),
            }

            // send decryption key
            self.conn
                .write_frame(UrsaFrame::DecryptionKeyResponse { decryption_key })
                .await?;
        }

        self.conn.write_frame(UrsaFrame::EndOfRequestSignal).await?;

        Ok(())
    }
}