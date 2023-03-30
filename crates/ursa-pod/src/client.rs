use std::{pin::Pin, task::Poll};

use bytes::{BufMut, Bytes, BytesMut};
use futures::{SinkExt, Stream, TryStreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tracing::{debug, info};

use crate::{
    codec::{UrsaCodec, UrsaCodecError, UrsaFrame},
    types::Blake3Cid,
};

const _PROOF_CHUNK_LEN: usize = 4 * 1024;
const CONTENT_CHUNK_LEN: usize = 16 * 1024;

/// UFDP Response struct
///
/// Implements [`Stream`], which can be wrapped with [`tokio_util::io::StreamReader`] for [`AsyncRead`].
///
/// Example:
#[cfg_attr(doctest, doc = " ````no_test")]
/// ```
/// let mut client = UfdpClient::new(stream).await?;
///
/// let mut reader = StreamReader::new(client.request(CID).await?);
/// let mut bytes = BytesMut::with_capacity(16 * 1024);
/// reader.read_buf(&mut bytes).await?;
///
/// println!("{bytes:?}");
/// ```
pub struct UfdpResponse<'client, S: AsyncRead + AsyncWrite + Unpin> {
    client: &'client mut UfdpClient<S>,
    current_block: BytesMut,
    index: u64,
    // todo: proof
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
                self.index += bytes.len() as u64;
                self.current_block.put_slice(&bytes);
                // todo: incremental verification

                if self.index < self.content_len {
                    Poll::Pending
                } else {
                    // block is ready
                    // todo:
                    //   - send DA
                    //   - recv decryption key
                    //   - decrypt block
                    Poll::Ready(Some(Ok(self.current_block.split().freeze())))
                }
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(_) => todo!("handle errors"),
            Poll::Pending => Poll::Pending,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.content_len as usize;
        (size, Some(size))
    }
}

/// UFDP Client. Accepts any stream of bytes supporting [`AsyncRead`] + [`AsyncWrite`]
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

    /// Send a request for content.
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
                debug!("received content response, streaming proof ({proof_len})");
                if proof_len != 0 {
                    unimplemented!()
                    // todo:
                    //  - stream and collect blake3tree bytes
                    //  - decode it
                    //  - pass to UfdpResponse to incrementally verify content
                }

                if content_len != 0 {
                    self.transport
                        .codec_mut()
                        .read_buffer(content_len as usize, CONTENT_CHUNK_LEN);

                    Ok(UfdpResponse {
                        client: self,
                        // todo: proof
                        current_block: BytesMut::with_capacity(256 * 1024),
                        index: 0,
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
