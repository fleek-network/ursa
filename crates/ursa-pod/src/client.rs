use bytes::{BufMut, BytesMut};

use tokio::io::{AsyncRead, AsyncWrite};

use tracing::debug;

use crate::{
    connection::{
        consts::{DECRYPTION_KEY_RES_TAG, HANDSHAKE_RES_TAG},
        UfdpConnection, UrsaCodecError, UrsaFrame,
    },
    instrument,
    types::{Blake3Cid, BlsPublicKey},
};

/// UFDP Client. Accepts any stream of bytes supporting [`AsyncRead`] + [`AsyncWrite`]
pub struct UfdpClient<S: AsyncRead + AsyncWrite + Unpin + Send + Sync> {
    conn: UfdpConnection<S>,
    lane: u8,
}

impl<S> UfdpClient<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    /// Create a new client, immediately attempting to handshake with the destination
    ///
    /// Accepts a stream implementing [`AsyncRead`] + [`AsyncWrite`],
    /// as well as the client's public key. If lane is none, then the server will select
    /// it automatically.
    pub async fn new(
        stream: S,
        pubkey: BlsPublicKey,
        lane: Option<u8>,
    ) -> Result<Self, UrsaCodecError> {
        let mut conn = UfdpConnection::new(stream);

        // Send handshake request.
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

        // Receive handshake response
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

        // Content response loop.
        loop {
            match instrument!(self.conn.read_frame(None).await?, "tag=read_content_res") {
                Some(UrsaFrame::ContentResponse {
                    proof_len,
                    block_len,
                    ..
                }) => {
                    // Receive proof
                    let len = proof_len as usize;
                    self.conn.read_buffer(len);
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
                            Some(_) => unreachable!(), // Guaranteed by read_buffer()
                            None => return Err(UrsaCodecError::Unknown),
                        }
                    }

                    // Receive block
                    let len = block_len as usize;
                    self.conn.read_buffer(len);
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
                            Some(_) => unreachable!(), // Guaranteed by read_buffer()
                            None => return Err(UrsaCodecError::Unknown),
                        }
                    }

                    // Send decryption key request
                    // todo: crypto integration
                    instrument!(
                        self.conn
                            .write_frame(UrsaFrame::DecryptionKeyRequest {
                                delivery_acknowledgment: [1; 96],
                            })
                            .await?,
                        "tag=write_dk_req"
                    );

                    // Receive decryption key
                    match instrument!(
                        self.conn.read_frame(Some(DECRYPTION_KEY_RES_TAG)).await?,
                        "tag=read_dk_res"
                    ) {
                        Some(UrsaFrame::DecryptionKeyResponse { .. }) => {
                            // todo: decrypt block & verify data
                        }
                        Some(_) => unreachable!(), // Guaranteed by frame filter
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

    /// Get the lane assigned to the client connection
    pub fn lane(&self) -> u8 {
        self.lane
    }
}
