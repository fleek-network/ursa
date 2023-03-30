use bytes::{BufMut, Bytes, BytesMut};
use futures::{ready, SinkExt, Stream, StreamExt, TryStreamExt};
use std::{
    pin::{pin, Pin},
    task::Poll,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;
use tracing::{debug, info};

use crate::{
    codec::{UrsaCodec, UrsaCodecError, UrsaFrame},
    types::Blake3Cid,
};

const _PROOF_CHUNK_LEN: usize = 4 * 1024;
const CONTENT_CHUNK_LEN: usize = 2;
const CONTENT_BLOCK_MAX: usize = 256 * 1024;

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
pub struct UfdpResponse<'client, S> {
    client: &'client mut UfdpClient<S>,
    current_block: BytesMut,
    // todo: proof
    block_len: usize,
    is_done: bool,
}

impl<'client, S> UfdpResponse<'client, S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub(crate) async fn new(
        client: &'client mut UfdpClient<S>,
    ) -> Result<UfdpResponse<S>, UrsaCodecError> {
        match client.transport.next().await.expect("content response")? {
            UrsaFrame::ContentResponse {
                content_len,
                proof_len,
                ..
            } => {
                debug!("received content response, streaming proof ({proof_len})");
                if proof_len != 0 {
                    unimplemented!()
                    // todo:
                    //  - stream and collect blake2tree bytes
                    //  - decode it
                    //  - pass to UfdpResponse to incrementally verify content
                }

                if content_len != 0 {
                    tracing::debug!("here {content_len}");

                    client
                        .transport
                        .codec_mut()
                        .read_buffer(content_len as usize, CONTENT_CHUNK_LEN);

                    Ok(UfdpResponse {
                        client,
                        // todo: proof
                        current_block: BytesMut::with_capacity(CONTENT_BLOCK_MAX),
                        block_len: content_len as usize,
                        is_done: false,
                    })
                } else {
                    Err(UrsaCodecError::Unknown)
                }
            }
            f => Err(UrsaCodecError::UnexpectedFrame(f.tag().unwrap())),
        }
    }
}

impl<'client, S> Stream for UfdpResponse<'client, S>
where
    S: AsyncRead + Unpin,
{
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        if self.is_done {
            return Poll::Ready(None);
        }

        // the only time we should be returning Poll::Pending from this function is when we receive
        // a Poll::Pending from the source. This is because when we return a Poll::Pending we
        // *must* ensure that the data current task will wake up after.

        loop {
            let res = self.client.transport.poll_next_unpin(cx);

            match ready!(res) {
                None => {
                    // What should we do with the data we might have gotten from previous iterations
                    // of the loop? Nothing. That data is useless because a partial block is useless.
                    //
                    // So technically this is an error at this layer.
                    self.is_done = true;
                    return Poll::Ready(Some(Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "Unexpected end of data when receiving content.",
                    ))));
                }
                Some(Err(_)) => {
                    self.is_done = true;
                    // TODO: impl From<UrsaCodecError> for std::io::Error.
                    todo!()
                }
                Some(Ok(UrsaFrame::Buffer(bytes))) => {
                    self.current_block.put_slice(&bytes);
                    // TODO: Do any incremental processing with the chunk.
                }
                Some(Ok(UrsaFrame::EndOfRequestSignal)) => {
                    self.is_done = true;
                    return Poll::Ready(Some(Ok(self.current_block.split().freeze())));
                }
                // TODO: Handle other cases.
                _ => todo!(),
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.block_len;
        // upper bound is none, since we're not sure how many blocks will get decoded in total
        (size, None)
    }
}

/// UFDP Client. Accepts any stream of bytes supporting [`AsyncRead`] + [`AsyncWrite`]
pub struct UfdpClient<S> {
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

        UfdpResponse::new(self).await
    }
}
