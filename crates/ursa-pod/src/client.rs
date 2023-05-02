use std::io::{Error, ErrorKind};

use blake3::{ursa::BlockHasher, Hash};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    connection::{
        consts::{DECRYPTION_KEY_RES_TAG, HANDSHAKE_RES_TAG},
        UfdpConnection, UrsaCodecError, UrsaFrame,
    },
    instrument,
    tree::IncrementalVerifier,
    types::BlsPublicKey,
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
            Some(_) => unreachable!(), // Gauranteed by filter
            None => Err(UrsaCodecError::Unknown),
        }
    }

    /// Send a request for content.
    pub async fn request(&mut self, hash: Hash) -> Result<usize, UrsaCodecError> {
        instrument!(
            self.conn
                .write_frame(UrsaFrame::ContentRequest { hash })
                .await?,
            "tag=write_content_req"
        );
        let mut size = 0;

        let mut verifier = IncrementalVerifier::new(hash.into(), 0);
        let mut block = 0;

        // Content response loop.
        loop {
            match instrument!(self.conn.read_frame(None).await?, "tag=read_content_res") {
                Some(UrsaFrame::ContentResponse {
                    proof_len,
                    block_len,
                    ..
                }) => {
                    // Receive proof
                    if proof_len != 0 {
                        self.conn.read_buffer(proof_len as usize);

                        match instrument!(
                            self.conn.read_frame(None).await?,
                            "tag=read_proof_buffer"
                        ) {
                            Some(UrsaFrame::Buffer(bytes)) => {
                                // Feed the verifier the proof
                                if let Err(e) = verifier.feed_proof(&bytes) {
                                    return Err(UrsaCodecError::Io(Error::new(
                                        ErrorKind::InvalidData,
                                        format!("feed_proof: {e}"),
                                    )));
                                }
                            }
                            Some(_) => unreachable!(), // Gauranteed by read_buffer()
                            None => return Err(UrsaCodecError::Unknown),
                        }
                    }

                    // Receive block
                    let len = block_len as usize;
                    self.conn.read_buffer(len);
                    size += len;

                    match instrument!(self.conn.read_frame(None).await?, "tag=read_block_buffer") {
                        Some(UrsaFrame::Buffer(bytes)) => {
                            // Verify data
                            let mut hasher = BlockHasher::new();
                            hasher.set_block(block);
                            hasher.update(&bytes);
                            if let Err(e) = verifier.verify(hasher) {
                                return Err(UrsaCodecError::Io(Error::new(
                                    ErrorKind::InvalidData,
                                    format!("{e}"),
                                )));
                            }
                        }
                        Some(_) => unreachable!(), // Guaranteed by read_buffer()
                        None => return Err(UrsaCodecError::Unknown),
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

                    block += 1;
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

    /// Get the lane assigned to the client connection
    pub fn lane(&self) -> u8 {
        self.lane
    }
}
