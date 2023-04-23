use std::sync::Arc;

use bytes::BytesMut;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    connection::{
        consts::{
            CONTENT_RANGE_REQ_TAG, CONTENT_REQ_TAG, DECRYPTION_KEY_REQ_TAG, HANDSHAKE_REQ_TAG,
        },
        UfdpConnection, UrsaCodecError, UrsaFrame,
    },
    instrument,
    types::{Blake3Cid, BlsSignature, Secp256k1AffinePoint, Secp256k1PublicKey},
};

/// Backend trait used by [`UfdpServer`] to access external data
pub trait Backend: Send + Sync + 'static {
    /// Get the raw content of a block.
    fn raw_block(&self, cid: &Blake3Cid, block: u64) -> Option<&[u8]>;

    /// Get a decryption_key for a block, includes a block request id.
    fn decryption_key(&self, request_id: u64) -> (Secp256k1AffinePoint, u64);

    /// Get a clients current balance.
    fn get_balance(&self, pubkey: Secp256k1PublicKey) -> u128;

    /// Save a batch of transactions to be submitted to consensus.
    fn save_batch(&self, batch: BlsSignature) -> Result<(), String>;
}

/// UFDP Handler.
///
/// Accepts any stream of data supporting [`AsyncWrite`] + [`AsyncRead`], and a backend.
pub struct UfdpHandler<S: AsyncRead + AsyncWrite + Unpin, B: Backend> {
    pub conn: UfdpConnection<S>,
    backend: Arc<B>,
    #[allow(unused)]
    session_id: u64,
}

impl<S: AsyncWrite + AsyncRead + Unpin, B: Backend> UfdpHandler<S, B> {
    #[inline(always)]
    pub fn new(stream: S, backend: B, session_id: u64) -> Self {
        Self {
            conn: UfdpConnection::new(stream),
            backend: Arc::new(backend),
            session_id,
        }
    }

    /// Begin serving a request. Accepts a handshake, and then begins the request loop.
    pub async fn serve(mut self) -> Result<(), UrsaCodecError> {
        // Step 1: Perform the handshake.
        instrument!(
            self.handshake().await?,
            "sid={},tag=handshake",
            self.session_id
        );

        // Step 2: Handle requests.
        while let Some(frame) = instrument!(
            self.conn
                .read_frame(Some(CONTENT_REQ_TAG | CONTENT_RANGE_REQ_TAG))
                .await?,
            "sid={},tag=read_content_req",
            self.session_id
        ) {
            match frame {
                UrsaFrame::ContentRequest { hash } => {
                    #[cfg(feature = "benchmarks")]
                    let (content_size, block_size) = {
                        let bytes = hash.0;
                        let block_size_bytes = arrayref::array_ref!(bytes, 0, 8);
                        let block_size = u64::from_be_bytes(*block_size_bytes);
                        let content_size_bytes = arrayref::array_ref!(bytes, 8, 8);
                        let content_size = u64::from_be_bytes(*content_size_bytes);
                        (content_size, block_size)
                    };

                    instrument!(
                        self.deliver_content(hash).await?,
                        "sid={},tag=deliver_content,content_size={content_size},block_size={block_size}",
                        self.session_id
                    );
                }
                UrsaFrame::ContentRangeRequest { .. } => todo!(),
                _ => unreachable!(),
            }
        }

        Ok(())
    }

    /// Wait and respond to a handshake request.
    #[inline(always)]
    pub async fn handshake(&mut self) -> Result<(), UrsaCodecError> {
        match instrument!(
            self.conn.read_frame(Some(HANDSHAKE_REQ_TAG)).await?,
            "sid={},tag=read_handshake_req",
            self.session_id
        ) {
            Some(UrsaFrame::HandshakeRequest { lane, .. }) => {
                // send res frame
                instrument!(
                    self.conn
                        .write_frame(UrsaFrame::HandshakeResponse {
                            pubkey: [2; 33],
                            epoch_nonce: 1000,
                            lane: lane.unwrap_or(0),
                            last: None,
                        })
                        .await?,
                    "sid={},tag=write_handshake_res",
                    self.session_id
                );

                Ok(())
            }
            None => Err(UrsaCodecError::Unknown),
            // handled by filter
            Some(_) => unreachable!(),
        }
    }

    /// Content delivery loop for a cid.
    #[inline(always)]
    pub async fn deliver_content(&mut self, cid: Blake3Cid) -> Result<(), UrsaCodecError> {
        #[cfg(feature = "benchmarks")]
        let (content_size, block_size) = {
            let bytes = cid.0;
            let block_size_bytes = arrayref::array_ref!(bytes, 0, 8);
            let block_size = u64::from_be_bytes(*block_size_bytes);
            let content_size_bytes = arrayref::array_ref!(bytes, 8, 8);
            let content_size = u64::from_be_bytes(*content_size_bytes);
            (content_size, block_size)
        };

        let mut block_number = 0;
        while let Some(block) = instrument!(
            self.backend.raw_block(&cid, block_number),
            "sid={},tag=backend_raw_block,content_size={content_size},block_size={block_size}",
            self.session_id
        ) {
            block_number += 1;

            // todo: integrate tree
            let proof = BytesMut::from(b"dummy_proof".as_slice());
            let decryption_key = [0; 33];
            let proof_len = proof.len() as u64;
            let block_len = block.len() as u64;

            instrument!(
                self.conn
                    .write_frame(UrsaFrame::ContentResponse {
                        compression: 0,
                        proof_len,
                        block_len,
                        signature: [1u8; 64],
                    })
                    .await?,
                "sid={},tag=write_content_res,content_size={content_size},block_size={block_size}",
                self.session_id
            );

            instrument!(
                self.conn.write_frame(UrsaFrame::Buffer(proof)).await?,
                "sid={},tag=write_proof,content_size={content_size},block_size={block_size}",
                self.session_id
            );
            instrument!(
                self.conn
                    .write_frame(UrsaFrame::Buffer(block.into()))
                    .await?,
                "sid={},tag=write_block,content_size={content_size},block_size={block_size}",
                self.session_id
            );

            // Wait for delivery acknowledgment
            match instrument!(
                self.conn.read_frame(Some(DECRYPTION_KEY_REQ_TAG)).await?,
                "sid={},tag=read_da,content_size={content_size},block_size={block_size}",
                self.session_id
            ) {
                Some(UrsaFrame::DecryptionKeyRequest { .. }) => {
                    // todo: transaction manager (batch and store tx)
                }
                None => return Err(UrsaCodecError::Unknown),
                Some(_) => unreachable!(),
            }

            // Send decryption key
            // todo: integrate crypto
            instrument!(
                self.conn
                    .write_frame(UrsaFrame::DecryptionKeyResponse { decryption_key })
                    .await?,
                "sid={},tag=write_dk,content_size={content_size},block_size={block_size}",
                self.session_id
            );
        }

        instrument!(
            self.conn.write_frame(UrsaFrame::EndOfRequestSignal).await?,
            "sid={},tag=write_eor,content_size={content_size},block_size={block_size}",
            self.session_id
        );

        Ok(())
    }
}
