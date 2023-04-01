use bytes::BytesMut;
use futures::SinkExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tracing::{debug, error};

use crate::{
    codec::{consts::MAX_BLOCK_SIZE, Reason, UrsaCodec, UrsaCodecError, UrsaFrame},
    types::{Blake3Cid, BlsSignature, Secp256k1AffinePoint, Secp256k1PublicKey},
};

const IO_CHUNK_SIZE: usize = 16 * 1024;

/// Backend trait used by [`UfdpServer`] to access external data
pub trait Backend: Copy + Send + Sync + 'static {
    /// Get some raw content for a given cid.
    /// Returns some raw bytes, and a request id to get the decryption_key
    fn raw_content(&self, cid: Blake3Cid) -> (BytesMut, u64);

    /// Get a decryption_key for a block, includes a block request id
    fn decryption_key(&self, request_id: u64) -> (Secp256k1AffinePoint, u64);

    /// Get a clients current balance.
    fn get_balance(&self, pubkey: Secp256k1PublicKey) -> u128;

    /// Save a batch of transactions to be submitted to consensus.
    fn save_batch(&self, batch: BlsSignature) -> Result<(), String>;
}

/// UFDP Server. Handles any stream of data supporting [`AsyncWrite`] + [`AsyncRead`]
pub struct UfdpServer<B: Backend> {
    backend: B,
}

impl<B> UfdpServer<B>
where
    B: Backend,
{
    pub fn new(backend: B) -> Result<Self, UrsaCodecError> {
        Ok(Self { backend })
    }

    /// Handle a connection. Spawns a tokio task and begins the session loop
    pub fn handle<S: AsyncWrite + AsyncRead + Unpin + Send + 'static>(
        &mut self,
        stream: S,
    ) -> Result<(), UrsaCodecError> {
        let backend = self.backend;
        tokio::spawn(async move {
            let mut transport = Framed::new(stream, UrsaCodec::default());

            match transport.next().await.expect("handshake request") {
                Ok(UrsaFrame::HandshakeRequest { lane, .. }) => {
                    debug!("Handshake received, sending response");
                    let lane = lane.unwrap_or({
                        // todo: lane management
                        0
                    });

                    transport
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

            debug!("Starting request loop");
            while let Some(frame) = transport.next().await {
                debug!("Received frame: {frame:?}");
                match frame {
                    Ok(UrsaFrame::ContentRequest { hash }) => {
                        debug!("Content request received");
                        let (mut content, request_id) = backend.raw_content(hash);
                        debug!("Sending content ({} bytes)", content.len());
                        while !content.is_empty() {
                            let block_len = content.len().min(MAX_BLOCK_SIZE);
                            let mut block = content.split_to(block_len);

                            let (decryption_key, _) = backend.decryption_key(request_id);

                            // todo: proof encoding
                            let proof = BytesMut::from(b"dummy_proof".as_slice());
                            let proof_len = proof.len() as u64;

                            debug!("Sending content response block");
                            transport
                                .send(UrsaFrame::ContentResponse {
                                    compression: 0,
                                    proof_len,
                                    block_len: block_len as u64,
                                    signature: [1u8; 64],
                                })
                                .await
                                .expect("send content response");

                            debug!("Sending proof ({proof_len} bytes)");
                            transport
                                .send(UrsaFrame::Buffer(proof))
                                .await
                                .expect("send proof data");

                            while !block.is_empty() {
                                let chunk_len = block.len().min(IO_CHUNK_SIZE);
                                debug!("Sending block chunk");
                                transport
                                    .send(UrsaFrame::Buffer(block.split_to(chunk_len)))
                                    .await
                                    .expect("send content data");
                            }

                            // wait for delivery acknowledgment
                            match transport.next().await {
                                Some(Ok(UrsaFrame::DecryptionKeyRequest { .. })) => {
                                    debug!("Delivery acknowledgment received");
                                    // todo: transaction manager (batch and store tx)
                                }
                                Some(Ok(f)) => error!("Unexpected frame {f:?}"),
                                Some(Err(e)) => error!("Codec error: {e:?}"),
                                None => error!("Connection closed"),
                            }

                            debug!("Sending decryption key");
                            // send decryption key
                            transport
                                .send(UrsaFrame::DecryptionKeyResponse { decryption_key })
                                .await
                                .expect("send decryption key");
                        }

                        debug!("Sending EOR");
                        transport
                            .send(UrsaFrame::EndOfRequestSignal)
                            .await
                            .expect("send EOR");
                        debug!("Waiting for next request");
                    }
                    Ok(f) => {
                        error!("Terminating, unexpected frame: {f:?}");
                        transport
                            .send(UrsaFrame::TerminationSignal(Reason::UnexpectedFrame))
                            .await
                            .expect("send termination signal");
                        drop(transport);
                        break;
                    }
                    Err(e) => {
                        error!("{e:?}");
                        transport
                            .send(UrsaFrame::TerminationSignal(Reason::Unknown))
                            .await
                            .expect("send termination signal");
                        drop(transport);
                        break;
                    }
                }
            }

            debug!("Connection Closed");
        });

        Ok(())
    }
}
