use bytes::{BufMut, BytesMut};

use tokio::io::{AsyncRead, AsyncWrite};

use tracing::debug;

use crate::{
    connection::{consts::HANDSHAKE_RES_TAG, UfdpConnection, UrsaCodecError, UrsaFrame},
    instrument,
    types::{Blake3Cid, BlsPublicKey},
};

/// UFDP Client. Accepts any stream of bytes supporting [`AsyncRead`] + [`AsyncWrite`]
pub struct UfdpClient<S: AsyncRead + AsyncWrite + Unpin + Send + Sync> {
    pub conn: UfdpConnection<S>,
    lane: u8,
}

impl<S> UfdpClient<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    /// Create a new client, attempting to handshake with the destination
    ///
    /// Accepts a stream implementing [`AsyncRead`] + [`AsyncWrite`],
    /// as well as the client's public key
    pub async fn new(
        stream: S,
        pubkey: BlsPublicKey,
        lane: Option<u8>,
    ) -> Result<Self, UrsaCodecError> {
        let mut conn = UfdpConnection::new(stream);

        // send handshake
        instrument!(
            conn.write_frame(UrsaFrame::HandshakeRequest {
                version: 0,
                supported_compression_bitmap: 0,
                lane,
                pubkey,
            })
            .await?,
            "tag=write_handshake_req"
        );

        // receive handshake
        match instrument!(
            conn.read_frame(Some(HANDSHAKE_RES_TAG)).await?,
            "tag=read_handshake_res"
        ) {
            Some(UrsaFrame::HandshakeResponse { lane, .. }) => Ok(Self { conn, lane }),
            Some(_) => unreachable!(),
            None => Err(UrsaCodecError::Unknown),
        }
    }

    /// Send a request for content.
    pub async fn request(&mut self, hash: Blake3Cid) -> Result<usize, UrsaCodecError> {
        instrument!(
            self.conn
                .write_frame(UrsaFrame::ContentRequest { hash })
                .await?,
            "tag=write_content_req"
        );
        let mut size = 0;

        loop {
            match instrument!(self.conn.read_frame(None).await?, "tag=read_content_res") {
                Some(UrsaFrame::ContentResponse {
                    proof_len,
                    block_len,
                    ..
                }) => {
                    // receive proof
                    let len = proof_len as usize;
                    self.conn.take = len;
                    let mut proof_buf = BytesMut::with_capacity(len);
                    loop {
                        match instrument!(
                            self.conn.read_frame(None).await?,
                            "tag=read_proof_buffer"
                        ) {
                            Some(UrsaFrame::Buffer(bytes)) => {
                                debug!("recv proof chunk");
                                proof_buf.put_slice(&bytes);
                                if proof_buf.len() == len {
                                    // todo: decode proof
                                    break;
                                }
                            }
                            Some(e) => {
                                return Err(UrsaCodecError::InvalidTag(e.tag().unwrap() as u8))
                            }
                            None => return Err(UrsaCodecError::Unknown),
                        }
                    }

                    // receive block
                    let len = block_len as usize;
                    self.conn.take = len;
                    let mut block_buf = BytesMut::with_capacity(len);
                    size += len;
                    loop {
                        match instrument!(
                            self.conn.read_frame(None).await?,
                            "tag=read_block_buffer"
                        ) {
                            Some(UrsaFrame::Buffer(bytes)) => {
                                block_buf.put_slice(&bytes);
                                if block_buf.len() == len {
                                    break;
                                }
                            }
                            Some(e) => {
                                return Err(UrsaCodecError::InvalidTag(e.tag().unwrap() as u8))
                            }
                            None => return Err(UrsaCodecError::Unknown),
                        }
                    }

                    // send decryption key request
                    instrument!(
                        self.conn
                            .write_frame(UrsaFrame::DecryptionKeyRequest {
                                delivery_acknowledgment: [1; 96],
                            })
                            .await?,
                        "tag=write_dk_req"
                    );

                    // receive decryption key
                    match instrument!(self.conn.read_frame(None).await?, "tag=read_dk_res") {
                        Some(UrsaFrame::DecryptionKeyResponse { .. }) => {}
                        _ => return Err(UrsaCodecError::Unknown),
                    }
                }
                Some(UrsaFrame::EndOfRequestSignal) => break,
                Some(f) => return Err(UrsaCodecError::InvalidTag(f.tag().unwrap() as u8)),
                None => return Err(UrsaCodecError::Unknown),
            }
        }

        Ok(size)
    }

    /// Consumes the client and returns the underlying stream.
    pub fn finish(self) -> S {
        self.conn.stream
    }

    /// Get the lane assigned to the connection
    pub fn lane(&self) -> u8 {
        self.lane
    }
}
