use bytes::BytesMut;
use futures::SinkExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tracing::{debug, info};

use crate::{
    codec::{UrsaCodec, UrsaCodecError, UrsaFrame},
    types::Blake3CID,
};

/// UFDP Client. Accepts any stream supporting [`AsyncRead`] + [`AsyncWrite`]
pub struct UfdpClient<S: AsyncRead + AsyncWrite + Unpin> {
    transport: Framed<S, UrsaCodec>,
}

impl<S> UfdpClient<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    /// Create a new client, attempting to handshake with the destination
    pub async fn new(stream: S) -> Result<Self, UrsaCodecError> {
        let codec = UrsaCodec::default();

        let mut transport = Framed::new(stream, codec);

        // send handshake
        transport
            .send(UrsaFrame::HandshakeRequest {
                version: 0,
                supported_compression_bitmap: 0,
                lane: 0xFF,
                pubkey: [1; 48],
            })
            .await
            .expect("handshake request");

        // receive handshake
        if let Ok(frame) = transport.next().await.expect("handshake response") {
            match frame {
                UrsaFrame::HandshakeResponse { .. } => {
                    info!("received handshake response from server: {frame:?}");
                }
                f => return Err(UrsaCodecError::UnexpectedFrame(f.tag().unwrap())),
            }
        }

        Ok(Self { transport })
    }

    /// Send a request for content
    pub async fn request(&mut self, hash: Blake3CID) -> Result<BytesMut, UrsaCodecError> {
        self.transport
            .send(UrsaFrame::ContentRequest { hash })
            .await
            .expect("content request");

        match self.transport.next().await.expect("content response")? {
            UrsaFrame::ContentResponse {
                content_len,
                proof_len,
                ..
            } => {
                info!("received content response");

                debug!("streaming proof ({proof_len})");
                if proof_len != 0 {
                    unimplemented!()
                }

                debug!("streaming content ({content_len})");
                if content_len != 0 {
                    self.transport
                        .codec_mut()
                        .read_buffer(content_len as usize, 16384);
                    match self.transport.next().await.expect("content buffer")? {
                        UrsaFrame::Buffer(bytes) => Ok(bytes),
                        f => Err(UrsaCodecError::UnexpectedFrame(f.tag().unwrap())),
                    }
                } else {
                    Err(UrsaCodecError::Unknown)
                }
            }
            f => Err(UrsaCodecError::UnexpectedFrame(f.tag().unwrap())),
        }
    }
}
