use std::{pin::Pin, task::Poll};

use bytes::{BufMut, Bytes, BytesMut};
use futures::{executor::block_on, ready, SinkExt, Stream, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;
use tracing::{debug, error};

use crate::{
    codec::{
        consts::{MAX_BLOCK_SIZE, MAX_PROOF_SIZE},
        UrsaCodec, UrsaCodecError, UrsaFrame,
    },
    types::{Blake3Cid, BlsPublicKey},
};

const IO_CHUNK_SIZE: usize = 16 * 1024;

#[derive(Clone, Copy, Debug)]
pub enum UfdpResponseState {
    WaitingForHeader,
    ReadingProof,
    ReadingContent,
    WaitingForDecryptionKey,
    Done,
}

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
pub struct UfdpResponse<'client, S: AsyncRead + AsyncWrite + Unpin + Send + Sync> {
    client: &'client mut UfdpClient<S>,
    current_proof: BytesMut,
    current_block: BytesMut,
    // todo: proof
    proof_len: usize,
    block_len: usize,
    state: UfdpResponseState,
}

impl<'client, S> UfdpResponse<'client, S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    pub(crate) fn new(client: &'client mut UfdpClient<S>) -> UfdpResponse<S> {
        UfdpResponse {
            client,
            current_proof: BytesMut::with_capacity(MAX_PROOF_SIZE),
            current_block: BytesMut::with_capacity(MAX_BLOCK_SIZE),
            proof_len: 0,
            block_len: 0,
            state: UfdpResponseState::WaitingForHeader,
        }
    }

    /// Get the current state of the response.
    pub fn state(&self) -> UfdpResponseState {
        self.state
    }
}

impl<'client, S> Stream for UfdpResponse<'client, S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        if let UfdpResponseState::Done = self.state {
            return Poll::Ready(None);
        }

        // the only time we should be returning Poll::Pending from this function is when we receive
        // a Poll::Pending from the source. This is because when we return a Poll::Pending we
        // *must* ensure that the data current task will wake up after.
        loop {
            let res = ready!(self.client.transport.poll_next_unpin(cx));
            match (self.state, res) {
                (
                    UfdpResponseState::WaitingForHeader,
                    Some(Ok(UrsaFrame::ContentResponse {
                        block_len,
                        proof_len,
                        ..
                    })),
                ) => {
                    self.block_len = block_len as usize;
                    let proof_len = proof_len as usize;
                    self.proof_len = proof_len;
                    self.client
                        .transport
                        .codec_mut()
                        .read_buffer(proof_len, IO_CHUNK_SIZE);
                    debug!("Received content block header");
                    self.state = UfdpResponseState::ReadingProof;
                }
                (UfdpResponseState::ReadingProof, Some(Ok(UrsaFrame::Buffer(bytes)))) => {
                    debug!("Received proof chunk");
                    self.current_proof.put_slice(&bytes);
                    if self.current_proof.len() == self.proof_len {
                        // todo: parse proof
                        let _proof_bytes = self.current_proof.split();
                        self.current_proof.reserve(MAX_PROOF_SIZE);

                        let block_len = self.block_len;
                        self.client
                            .transport
                            .codec_mut()
                            .read_buffer(block_len, IO_CHUNK_SIZE);
                        debug!("Finished reading proof, reading block {block_len}");
                        self.state = UfdpResponseState::ReadingContent
                    }
                }
                (UfdpResponseState::ReadingContent, Some(Ok(UrsaFrame::Buffer(bytes)))) => {
                    self.current_block.put_slice(&bytes);
                    // TODO: Do any incremental processing with the chunk.

                    if self.current_block.len() == self.block_len {
                        // BLOCKING: send delivery acknowledgment
                        debug!("Sending decryption key request");
                        block_on(self.client.transport.send(UrsaFrame::DecryptionKeyRequest {
                            delivery_acknowledgment: [1; 96],
                        }))
                        .expect("send delivery acknowledgment");

                        // wait for decryption key
                        debug!("Waiting for decryption key");
                        self.state = UfdpResponseState::WaitingForDecryptionKey;
                    }
                }
                (
                    UfdpResponseState::WaitingForDecryptionKey,
                    Some(Ok(UrsaFrame::DecryptionKeyResponse { .. })),
                ) => {
                    // todo: decrypt block
                    debug!("Received decryption key");
                    self.state = UfdpResponseState::WaitingForHeader;
                    return Poll::Ready(Some(Ok(self.current_block.split().freeze())));
                }
                (_, Some(Ok(UrsaFrame::EndOfRequestSignal))) => {
                    debug!("Received end of request signal");
                    self.state = UfdpResponseState::Done;
                    return Poll::Ready(None);
                }
                (s, Some(Err(e))) => {
                    self.state = UfdpResponseState::Done;
                    // TODO: impl From<UrsaCodecError> for std::io::Error.
                    error!("state: {s:?}, error: {e:?}");
                    todo!()
                }
                (_, None) => {
                    // What should we do with the data we might have gotten from previous iterations
                    // of the loop? Nothing. That data is useless because a partial block is useless.
                    //
                    // So technically this is an error at this layer.
                    self.state = UfdpResponseState::Done;
                    return Poll::Ready(Some(Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "Unexpected end of data when receiving content.",
                    ))));
                }

                // TODO: Handle other cases.
                _ => todo!(),
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // upper bound is none, since we're not sure how many blocks will get decoded in total
        (self.block_len, None)
    }
}

/// UFDP Client. Accepts any stream of bytes supporting [`AsyncRead`] + [`AsyncWrite`]
pub struct UfdpClient<S: AsyncRead + AsyncWrite + Unpin + Send + Sync> {
    transport: Framed<S, UrsaCodec>,
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
        let mut transport = Framed::new(stream, UrsaCodec::default());

        // send handshake
        debug!("Sending handshake request");
        transport
            .send(UrsaFrame::HandshakeRequest {
                version: 0,
                supported_compression_bitmap: 0,
                lane,
                pubkey,
            })
            .await
            .expect("handshake request");

        // receive handshake
        debug!("Received handshake request");
        match transport.next().await.expect("handshake response") {
            Ok(UrsaFrame::HandshakeResponse { lane, .. }) => {
                debug!("Received handshake response from server");
                Ok(Self { transport, lane })
            }
            Ok(f) => Err(UrsaCodecError::UnexpectedFrame(f.tag().unwrap())),
            Err(e) => Err(e),
        }
    }

    /// Send a request for content.
    pub async fn request(&mut self, hash: Blake3Cid) -> Result<UfdpResponse<S>, UrsaCodecError> {
        debug!("Sending content request");
        self.transport
            .send(UrsaFrame::ContentRequest { hash })
            .await?;

        Ok(UfdpResponse::new(self))
    }

    /// Get the lane assigned to the connection
    pub fn lane(&self) -> u8 {
        self.lane
    }
}
