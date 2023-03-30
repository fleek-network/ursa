use bytes::BytesMut;
use futures::SinkExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tracing::{debug, error, info};

use crate::{
    codec::{UrsaCodec, UrsaCodecError, UrsaFrame},
    types::{Blake3Cid, BlsSignature, Secp256k1PublicKey},
};

/// Backend trait used by [`UfdpServer`] to access external data
pub trait Backend: Copy + Send + Sync + 'static {
    /// Get some raw content for a given cid
    fn raw_content(&self, cid: Blake3Cid) -> BytesMut;

    /// Get a users balance
    fn get_balance(&self, pubkey: Secp256k1PublicKey) -> u128;

    /// Save a transaction to be batched and submitted
    fn save_tx(
        &self,
        pubkey: Secp256k1PublicKey,
        acknowledgment: BlsSignature,
    ) -> Result<(), String>;
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

    /// Handle a connection. Spawns a tokio task and begins the connection loop
    pub fn handle<S: AsyncWrite + AsyncRead + Unpin + Send + 'static>(
        &mut self,
        stream: S,
    ) -> Result<(), UrsaCodecError> {
        let backend = self.backend;
        tokio::spawn(async move {
            let mut transport = Framed::new(stream, UrsaCodec::default());

            match transport.next().await.expect("handshake request") {
                Ok(UrsaFrame::HandshakeRequest { lane, .. }) => {
                    info!("Handshake received, sending response");
                    transport
                        .send(UrsaFrame::HandshakeResponse {
                            pubkey: [2; 33],
                            epoch_nonce: 1000,
                            lane: if lane == 0xFF { 0 } else { lane },
                            last: None,
                        })
                        .await
                        .expect("handshake response");
                }
                _ => return,
            }

            while let Some(request) = transport.next().await {
                debug!("Received frame: {request:?}");
                match request {
                    Ok(UrsaFrame::ContentRequest { hash }) => {
                        info!("Content request received, sending response");
                        let content = backend.raw_content(hash);
                        transport
                            .send(UrsaFrame::ContentResponse {
                                compression: 0,
                                proof_len: 0,
                                content_len: content.len() as u64,
                                signature: [1u8; 64],
                            })
                            .await
                            .expect("content response");

                        transport
                            .send(UrsaFrame::Buffer(content))
                            .await
                            .expect("content data")
                    }
                    Ok(_) => unimplemented!(),
                    Err(e) => {
                        error!("{e:?}");
                        break;
                    }
                }
            }

            debug!("Connection Closed");
        });
        Ok(())
    }
}
