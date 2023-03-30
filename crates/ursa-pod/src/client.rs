use std::{pin::Pin, task::Poll};
//use std::task::Poll;

use bytes::{Bytes, BytesMut};
use futures::{SinkExt, Stream, TryStreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tracing::{debug, info};

use crate::{
    codec::{UrsaCodec, UrsaCodecError, UrsaFrame},
    types::Blake3Cid,
};

const PROOF_CHUNK_LEN: usize = 4 * 1024;
const CONTENT_CHUNK_LEN: usize = 16 * 1024;

/// UFDP Response, implementing [`AsyncRead`] to stream chunks of content
pub struct UfdpResponse<'client, S: AsyncRead + AsyncWrite + Unpin> {
    client: &'client mut UfdpClient<S>,
    _proof: BytesMut,
    pub content_len: u64,
}

impl<'client, S> Stream for UfdpResponse<'client, S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.client.transport.try_poll_next_unpin(cx) {
            Poll::Ready(Some(Ok(UrsaFrame::Buffer(bytes)))) => {
                // todo: verify/decode bytes
                Poll::Ready(Some(Ok(bytes.freeze())))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(_) => todo!("handle errors"),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// UFDP Client. Accepts any stream supporting [`AsyncRead`] + [`AsyncWrite`]
pub struct UfdpClient<S: AsyncRead + AsyncWrite + Unpin> {
    pub transport: Framed<S, UrsaCodec>,
}

impl<S> UfdpClient<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    /// Create a new client, attempting to handshake with the destination
    pub async fn new(stream: S) -> Result<Self, UrsaCodecError> {
        let mut transport = Framed::new(stream, UrsaCodec::default());

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
    pub async fn request(&mut self, hash: Blake3Cid) -> Result<UfdpResponse<S>, UrsaCodecError> {
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
                // todo: blake3tree encode/decode
                if proof_len != 0 {
                    unimplemented!()
                }

                debug!("streaming content ({content_len})");
                if content_len != 0 {
                    self.transport
                        .codec_mut()
                        .read_buffer(content_len as usize, 16 * 1024);

                    Ok(UfdpResponse {
                        client: self,
                        // todo: proof
                        _proof: BytesMut::new(),
                        content_len,
                    })
                } else {
                    Err(UrsaCodecError::Unknown)
                }
            }
            f => Err(UrsaCodecError::UnexpectedFrame(f.tag().unwrap())),
        }
    }
}
