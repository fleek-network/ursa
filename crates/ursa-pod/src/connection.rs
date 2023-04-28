use std::io::{Error, ErrorKind, Write};

use arrayref::array_ref;
use arrayvec::ArrayVec;
use bytes::BytesMut;
use consts::*;
use futures::executor::block_on;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::types::{
    Blake3Cid, BlsPublicKey, BlsSignature, EpochNonce, SchnorrSignature, Secp256k1AffinePoint,
    Secp256k1PublicKey,
};

/// Constant values for the codec.
pub mod consts {
    /// Network byte prefix in [`super::UrsaFrame::HandshakeRequest`]
    pub const NETWORK: [u8; 4] = *b"URSA";
    /// Maximum size for a frame
    pub const MAX_FRAME_SIZE: usize = 1024;
    /// Maximum lanes a client can use at one time
    pub const MAX_LANES: u8 = 24;
    /// Maximum bytes a proof can be.
    ///
    /// The maximum theoretical file we support is `2^64` bytes, given we transfer
    /// data as blocks of 256KiB (`2^18` bytes) the maximum number of chunks is `2^46`.
    /// So the maximum height of the hash tree will be 47. So we will have maximum of
    /// 47 hashes (hence `47 * 32`), and one byte per each 8 hash (`ceil(47 / 8) = 6`).
    pub const MAX_PROOF_SIZE: usize = 47 * 32 + 6;
    /// Maximum bytes a block can be
    pub const MAX_BLOCK_SIZE: usize = 4 * 256 * 1024;

    /// The bit flag on any frame tag sent from the node to the client.
    pub const IS_RES_FLAG: u8 = 0b10000000;

    /// [`super::UrsaFrame::HandshakeRequest`]
    pub const HANDSHAKE_REQ_TAG: u8 = 0x01 << 0;
    /// [`super::UrsaFrame::HandshakeResponse`]
    pub const HANDSHAKE_RES_TAG: u8 = IS_RES_FLAG | HANDSHAKE_REQ_TAG;
    /// [`super::UrsaFrame::ContentRequest`]
    pub const CONTENT_REQ_TAG: u8 = 0x01 << 1;
    /// [`super::UrsaFrame::ContentRangeRequest`]
    pub const CONTENT_RANGE_REQ_TAG: u8 = 0x01 << 2;
    /// [`super::UrsaFrame::ContentResponse`]
    pub const CONTENT_RES_TAG: u8 = IS_RES_FLAG | CONTENT_REQ_TAG;
    /// [`super::UrsaFrame::DecryptionKeyRequest`]
    pub const DECRYPTION_KEY_REQ_TAG: u8 = 0x01 << 3;
    /// [`super::UrsaFrame::DecryptionKeyResponse`]
    pub const DECRYPTION_KEY_RES_TAG: u8 = IS_RES_FLAG | DECRYPTION_KEY_REQ_TAG;

    /// [`super::UrsaFrame::UpdateEpochSignal`]
    pub const UPDATE_EPOCH_SIGNAL_TAG: u8 = IS_RES_FLAG | (0x01 << 4);
    /// [`super::UrsaFrame::EndOfRequestSignal`]
    pub const END_OF_REQUEST_SIGNAL_TAG: u8 = IS_RES_FLAG | (0x01 << 5);
    /// [`super::UrsaFrame::TerminationSignal`]
    pub const TERMINATATION_SIGNAL_TAG: u8 = IS_RES_FLAG | (0x01 << 6);

    /// Snappy compression bitmap value
    pub const SNAPPY: u8 = 0x01;
    /// GZip compression bitmap value
    pub const GZIP: u8 = 0x01 << 2;
    /// LZ4 compression bitmap value
    pub const LZ4: u8 = 0x01 << 3;
}

/// Termination reasons
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reason {
    UnexpectedFrame = 0x00,
    InsufficientBalance = 0x01,
    Unknown = 0xFF,
}

impl Reason {
    fn from_u8(byte: u8) -> Option<Self> {
        match byte {
            0x00 => Some(Self::UnexpectedFrame),
            0x01 => Some(Self::InsufficientBalance),
            0xFF => Some(Self::Unknown),
            _ => None,
        }
    }
}

/// Last known data for a lane
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LastLaneData {
    pub bytes: u64,
    pub signature: BlsSignature,
}

/// Frame tags
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameTag {
    HandshakeRequest = HANDSHAKE_REQ_TAG,
    HandshakeResponse = HANDSHAKE_RES_TAG,
    ContentRequest = CONTENT_REQ_TAG,
    ContentRangeRequest = CONTENT_RANGE_REQ_TAG,
    ContentResponse = CONTENT_RES_TAG,
    DecryptionKeyRequest = DECRYPTION_KEY_REQ_TAG,
    DecryptionKeyResponse = DECRYPTION_KEY_RES_TAG,
    UpdateEpochSignal = UPDATE_EPOCH_SIGNAL_TAG,
    EndOfRequestSignal = END_OF_REQUEST_SIGNAL_TAG,
    TerminationSignal = TERMINATATION_SIGNAL_TAG,
}

impl FrameTag {
    #[inline(always)]
    pub fn size_hint(&self) -> usize {
        match self {
            FrameTag::HandshakeRequest => 56,
            FrameTag::HandshakeResponse => 44,
            FrameTag::ContentRequest => 33,
            FrameTag::ContentResponse => 82, // header only
            FrameTag::ContentRangeRequest => 43,
            FrameTag::DecryptionKeyRequest => 97,
            FrameTag::DecryptionKeyResponse => 34,
            FrameTag::UpdateEpochSignal => 9,
            FrameTag::EndOfRequestSignal => 1,
            FrameTag::TerminationSignal => 2,
        }
    }
}

impl TryFrom<u8> for FrameTag {
    type Error = UrsaCodecError;

    #[inline(always)]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            HANDSHAKE_REQ_TAG => Ok(Self::HandshakeRequest),
            HANDSHAKE_RES_TAG => Ok(Self::HandshakeResponse),
            CONTENT_REQ_TAG => Ok(Self::ContentRequest),
            CONTENT_RES_TAG => Ok(Self::ContentResponse),
            CONTENT_RANGE_REQ_TAG => Ok(Self::ContentRangeRequest),
            DECRYPTION_KEY_REQ_TAG => Ok(Self::DecryptionKeyRequest),
            DECRYPTION_KEY_RES_TAG => Ok(Self::DecryptionKeyResponse),
            UPDATE_EPOCH_SIGNAL_TAG => Ok(Self::UpdateEpochSignal),
            END_OF_REQUEST_SIGNAL_TAG => Ok(Self::EndOfRequestSignal),
            TERMINATATION_SIGNAL_TAG => Ok(Self::TerminationSignal),
            t => Err(UrsaCodecError::InvalidTag(t)),
        }
    }
}

/// Frame variants for different requests and responses
///
/// All frames are prefixed with a [`FrameTag`].
///
/// Signals are not a response to a particular request, but they still have the `IS_RES` bit
/// enabled since they are sent from the node to the client.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UrsaFrame {
    /// Client request to initiate a UFDP connection.
    ///
    /// Clients can optionally resume a previous lane in the event of a disconnection.
    /// To let the node select the lane automatically, `lane` should be set to `0xFF`.
    ///
    /// ```text
    /// [ TAG . b"URSA" . version (1) . supported compression algorithm bitmap (1) . session lane . pubkey (48) ]
    /// ```
    /// size: 56 bytes
    HandshakeRequest {
        version: u8,
        supported_compression_bitmap: u8,
        lane: Option<u8>,
        pubkey: BlsPublicKey,
    },
    /// Node response to confirm a UFDP connection.
    ///
    /// Node will set a lane if unspecified by the client, or reuse an existing lane, including the [`LastLaneData`].
    ///
    /// ```text
    /// [ TAG . lane (1) . epoch nonce (8) . pubkey (32) ] [ 0x00 (1) || 0x80 (1) . u64 (8) . bls signature (96) ]
    /// ```
    /// size: 44 bytes or 147 bytes
    HandshakeResponse {
        pubkey: Secp256k1PublicKey,
        epoch_nonce: EpochNonce,
        lane: u8,
        last: Option<LastLaneData>,
    },
    /// Client request for content
    ///
    /// ```text
    /// [ TAG . blake3hash (32) ]
    /// ```
    /// size: 33 bytes
    ContentRequest { hash: Blake3Cid },
    /// Node response for content.
    ///
    /// The frame is always followed by the raw proof and content bytes.
    ///
    /// ```text
    /// [ TAG . compression (1) . proof length (8) . block length (8) . signature (64) ] [ proof .. ] [ content .. ]
    /// ```
    /// size: 82 bytes + proof len (max 16KB) + content len (max 256KB)
    ContentResponse {
        compression: u8,
        proof_len: u64,
        block_len: u64,
        signature: SchnorrSignature,
    },
    /// Not a frame. Buffer contains a chunk of bytes initiated after the [`UfdpConnection::read_buffer`] method has been called.
    /// It does *not* have a tag, and is used to chunk bytes after a [`UrsaFrame::ContentResponse`].
    Buffer(BytesMut),
    /// Client request for a range of chunks of content
    ///
    /// ```text
    /// [ TAG . blake3hash (32) . u64 (8) . u16 (2) ]
    /// ```
    /// size: 43 bytes
    ContentRangeRequest {
        hash: Blake3Cid,
        chunk_start: u64,
        chunks: u16,
    },
    /// Client request for a decryption key.
    /// The BLS delivery acknowledgment is batched and submitted by the node for rewards
    ///
    /// ```text
    /// [ TAG . bls signature (96) ]
    /// ```
    /// size: 97 bytes
    DecryptionKeyRequest {
        delivery_acknowledgment: BlsSignature,
    },
    /// Node response for a decryption key.
    /// The client will use the point to decrypt their data and use it.
    ///
    /// ```text
    /// [ TAG . decryption key (33) ]
    /// ```
    /// size: 34 bytes
    DecryptionKeyResponse {
        decryption_key: Secp256k1AffinePoint,
    },
    /// Signal from the node an epoch has changed during a connection.
    /// Clients should sign the next delivery acknowledgments with this new epoch.
    ///
    /// ```text
    /// [ TAG . epoch nonce (8) ]
    /// ```
    /// size: 9 bytes
    UpdateEpochSignal(EpochNonce),
    /// Signal from the node the request is finished and no more blocks will be sent
    ///
    /// ```text
    /// [ TAG ]
    /// ```
    /// size: 1 byte
    EndOfRequestSignal,
    /// Signal from the node the connection was terminated, with a reason.
    ///
    /// ```text
    /// [ TAG . reason (1) ]
    /// ```
    /// size: 2 bytes
    TerminationSignal(Reason),
}

impl UrsaFrame {
    /// Return the frame's tag or `None` if the frame is a `Buffer`.
    #[inline(always)]
    pub fn tag(&self) -> Option<FrameTag> {
        match self {
            Self::HandshakeRequest { .. } => Some(FrameTag::HandshakeRequest),
            Self::HandshakeResponse { .. } => Some(FrameTag::HandshakeResponse),
            Self::ContentRequest { .. } => Some(FrameTag::ContentRequest),
            Self::ContentRangeRequest { .. } => Some(FrameTag::ContentRangeRequest),
            Self::ContentResponse { .. } => Some(FrameTag::ContentResponse),
            Self::DecryptionKeyRequest { .. } => Some(FrameTag::DecryptionKeyRequest),
            Self::DecryptionKeyResponse { .. } => Some(FrameTag::DecryptionKeyResponse),
            Self::UpdateEpochSignal(_) => Some(FrameTag::UpdateEpochSignal),
            Self::EndOfRequestSignal => Some(FrameTag::EndOfRequestSignal),
            Self::TerminationSignal(_) => Some(FrameTag::TerminationSignal),
            Self::Buffer(_) => None,
        }
    }

    /// Return an estimation of the number of bytes this frame will need.
    #[inline]
    pub fn size_hint(&self) -> usize {
        match self {
            Self::Buffer(buffer) => buffer.len(),
            // SAFETY: unwrap is safe since the only time `tag` returns `None`
            // is when we have a `Self::Buffer` which we have already taken
            // care of.
            _ => self.tag().unwrap().size_hint(),
        }
    }
}

#[derive(Debug)]
pub enum UrsaCodecError {
    InvalidNetwork,
    InvalidTag(u8),
    InvalidReason(u8),
    UnexpectedFrame(FrameTag),
    ZeroLengthBlock,
    Io(std::io::Error),
    Unknown,
}

impl From<std::io::Error> for UrsaCodecError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<UrsaCodecError> for std::io::Error {
    fn from(value: UrsaCodecError) -> Self {
        match value {
            UrsaCodecError::Io(e) => e,
            error => Error::new(ErrorKind::Other, format!("{error:?}")),
        }
    }
}

/// Ursa Fair Delivery Codec for tokio's [`Encoder`] and [`Decoder`] traits.
pub struct UfdpConnection<R: AsyncRead + Unpin + Send + Sync, W: AsyncWrite + Unpin + Send + Sync> {
    pub read_half: UfdpConnectionReadHalf<R>,
    pub write_half: UfdpConnectionWriteHalf<W>,
}

pub struct UfdpConnectionWriteHalf<W: AsyncWrite + Unpin + Send + Sync> {
    pub write_stream: W,
}

pub struct UfdpConnectionReadHalf<R: AsyncRead + Unpin + Send + Sync> {
    pub read_stream: R,
    buffer: BytesMut,
    pub take: usize,
}

impl<W: AsyncWrite + Unpin + Send + Sync> From<W> for UfdpConnectionWriteHalf<W> {
    fn from(write_stream: W) -> Self {
        Self { write_stream }
    }
}

impl<R: AsyncRead + Unpin + Send + Sync> From<R> for UfdpConnectionReadHalf<R> {
    fn from(read_stream: R) -> Self {
        Self {
            read_stream,
            // Our max frame size is 148, and the only instance of multiple frames back to back
            // is within that size (ContentResponse + EoR), so maintaining at least 148 bytes is
            // enough to always be able to read the next incoming frame(s) in one pass before we
            // need to respond. For proof and block buffers, read_buffer is called before hand,
            // which will ensure we have enough space to read the entire chunk at once.
            buffer: BytesMut::with_capacity(148),
            take: 0,
        }
    }
}

impl<R> UfdpConnectionReadHalf<R>
where
    R: AsyncRead + Unpin + Send + Sync,
{
    /// Indicate the next `len` bytes are to be returned as a raw buffer. Always called after a content
    /// response, to receive the raw proof and block bytes.
    #[inline(always)]
    pub fn read_buffer(&mut self, len: usize) {
        self.take = len;
        // Ensure we have enough space to read the entire chunk at once if possible.
        self.buffer.reserve(len);
    }

    #[inline(always)]
    pub async fn read_frame(&mut self, filter: Option<u8>) -> std::io::Result<Option<UrsaFrame>> {
        loop {
            // If we have a full frame, parse and return it.
            if let Some(frame) = self.parse_frame(filter)? {
                return Ok(Some(frame));
            }

            // Otherwise, read as many bytes as we can for a fixed frame.
            if 0 == self.read_stream.read_buf(&mut self.buffer).await? {
                // Handle connection closed. If there are bytes in the buffer, it means the
                // connection was interrupted mid-transmission.
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err(Error::new(
                        ErrorKind::ConnectionReset,
                        "Client disconnected",
                    ));
                }
            }
        }
    }

    #[inline(always)]
    fn parse_frame(&mut self, filter: Option<u8>) -> std::io::Result<Option<UrsaFrame>> {
        let len = self.buffer.len();
        if len == 0 {
            return Ok(None);
        }

        // Are we reading raw bytes?
        if self.take > 0 {
            // Return as many bytes as we can.
            let take = len.min(self.take);
            self.take -= take;
            return Ok(Some(UrsaFrame::Buffer(self.buffer.split_to(take))));
        }

        // First frame byte is always the tag.
        let (size_hint, tag) = FrameTag::try_from(self.buffer[0]).map(|t| (t.size_hint(), t))?;

        if let Some(bitmap) = filter {
            let val = tag as u8;
            if val & bitmap != val {
                // Parsing frame should not decide to write anything - this logic needs to be
                // safely moved somewhere else.
                //
                // block_on(async {
                //     self.termination_signal(Some(Reason::UnexpectedFrame))
                //         .await
                //         .ok() // We dont care about this result!
                // });
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("Invalid tag: {tag:?}"),
                ));
            }
        }

        if len < size_hint {
            return Ok(None);
        }

        // We're going to take the frame's length, so lets reserve the amount for the next frame.
        self.buffer.reserve(tag.size_hint());

        match tag {
            FrameTag::HandshakeRequest => {
                let buf = self.buffer.split_to(size_hint);
                let network = &buf[1..5];
                if network != NETWORK {
                    return Err(UrsaCodecError::InvalidNetwork.into());
                }

                let version = buf[5];
                let supported_compression_bitmap = buf[6];
                let lane = match buf[7] {
                    0xFF => None,
                    v => Some(v),
                };
                let pubkey = *array_ref!(buf, 8, 48);

                Ok(Some(UrsaFrame::HandshakeRequest {
                    version,
                    supported_compression_bitmap,
                    lane,
                    pubkey,
                }))
            }
            FrameTag::HandshakeResponse => {
                // The 43rd byte of a handshake response identifies if the buffer includes the
                // last lane data or not.
                let (buf, last) = match self.buffer[43] {
                    0x80 => {
                        // Frame data is present.
                        let size_hint = size_hint + 104;
                        if len < size_hint {
                            return Ok(None);
                        }

                        let buf = self.buffer.split_to(size_hint);
                        let bytes_bytes = *array_ref!(buf, 44, 8);
                        let bytes = u64::from_be_bytes(bytes_bytes);
                        let signature = *array_ref!(buf, 52, 96);
                        (buf, Some(LastLaneData { bytes, signature }))
                    }
                    // todo: double check this
                    0x00 => (self.buffer.split_to(size_hint), None),
                    _ => return Err(UrsaCodecError::Unknown.into()),
                };
                let lane = buf[1];
                let epoch_nonce_bytes = *array_ref!(buf, 2, 8);
                let epoch_nonce = u64::from_be_bytes(epoch_nonce_bytes);
                let pubkey = *array_ref!(buf, 10, 33);

                Ok(Some(UrsaFrame::HandshakeResponse {
                    pubkey,
                    epoch_nonce,
                    lane,
                    last,
                }))
            }
            FrameTag::ContentRequest => {
                let buf = self.buffer.split_to(size_hint);
                let hash = Blake3Cid(*array_ref!(buf, 1, 32));

                Ok(Some(UrsaFrame::ContentRequest { hash }))
            }
            FrameTag::ContentResponse => {
                let buf = self.buffer.split_to(size_hint);
                let compression = buf[1];
                let proof_len_bytes = *array_ref!(buf, 2, 8);
                let proof_len = u64::from_be_bytes(proof_len_bytes);
                let block_len_bytes = *array_ref!(buf, 10, 8);
                let block_len = u64::from_be_bytes(block_len_bytes);
                if block_len == 0 {
                    return Err(UrsaCodecError::ZeroLengthBlock.into());
                }
                let signature = *array_ref!(buf, 18, 64);

                Ok(Some(UrsaFrame::ContentResponse {
                    compression,
                    proof_len,
                    block_len,
                    signature,
                }))
            }
            FrameTag::ContentRangeRequest => {
                let buf = self.buffer.split_to(size_hint);
                let hash = Blake3Cid(*array_ref!(buf, 1, 32));
                let chunk_start_bytes = *array_ref!(buf, 33, 8);
                let chunk_start = u64::from_be_bytes(chunk_start_bytes);
                let chunks = u16::from_be_bytes([buf[41], buf[42]]);

                Ok(Some(UrsaFrame::ContentRangeRequest {
                    hash,
                    chunk_start,
                    chunks,
                }))
            }
            FrameTag::DecryptionKeyRequest => {
                let buf = self.buffer.split_to(size_hint);
                let delivery_acknowledgment = *array_ref!(buf, 1, 96);

                Ok(Some(UrsaFrame::DecryptionKeyRequest {
                    delivery_acknowledgment,
                }))
            }
            FrameTag::DecryptionKeyResponse => {
                let buf = self.buffer.split_to(size_hint);
                let decryption_key = *array_ref!(buf, 1, 33);

                Ok(Some(UrsaFrame::DecryptionKeyResponse { decryption_key }))
            }
            FrameTag::UpdateEpochSignal => {
                let buf = self.buffer.split_to(size_hint);
                let epoch_bytes = *array_ref!(buf, 1, 8);
                let epoch_nonce = u64::from_be_bytes(epoch_bytes);

                Ok(Some(UrsaFrame::UpdateEpochSignal(epoch_nonce)))
            }
            FrameTag::EndOfRequestSignal => {
                let _ = self.buffer.split_to(1);
                Ok(Some(UrsaFrame::EndOfRequestSignal))
            }
            FrameTag::TerminationSignal => {
                let buf = self.buffer.split_to(size_hint);
                let byte = buf[1];

                if let Some(reason) = Reason::from_u8(byte) {
                    Ok(Some(UrsaFrame::TerminationSignal(reason)))
                } else {
                    Err(UrsaCodecError::InvalidReason(byte).into())
                }
            }
        }
    }
}

impl<W> UfdpConnectionWriteHalf<W>
where
    W: AsyncWrite + Unpin + Send + Sync,
{
    #[inline(always)]
    pub async fn write_frame(&mut self, frame: UrsaFrame) -> std::io::Result<()> {
        match frame {
            UrsaFrame::Buffer(bytes) => {
                // write directly to stream
                self.write_stream.write_all(&bytes).await?;
            }
            UrsaFrame::HandshakeRequest {
                version,
                pubkey,
                supported_compression_bitmap,
                lane,
            } => {
                let mut buf = ArrayVec::<u8, 56>::new_const();
                debug_assert_eq!(NETWORK.len(), 4);

                buf.push(FrameTag::HandshakeRequest as u8);
                buf.write_all(&NETWORK).unwrap();
                buf.push(version);
                buf.push(supported_compression_bitmap);
                buf.push(lane.unwrap_or(0xFF));
                buf.write_all(&pubkey).unwrap();

                self.write_stream.write_all(&buf).await?;
            }
            UrsaFrame::HandshakeResponse {
                pubkey,
                epoch_nonce,
                lane,
                last,
            } => {
                let mut buf = ArrayVec::<u8, 148>::new_const();

                buf.push(FrameTag::HandshakeResponse as u8);
                buf.push(lane);
                buf.write_all(&epoch_nonce.to_be_bytes()).unwrap();
                buf.write_all(&pubkey).unwrap();

                match last {
                    None => buf.push(0x00),
                    Some(data) => {
                        buf.push(0x80);
                        buf.write_all(&data.bytes.to_be_bytes()).unwrap();
                        buf.write_all(&data.signature).unwrap()
                    }
                };

                self.write_stream.write_all(&buf).await?;
            }
            UrsaFrame::ContentRequest { hash } => {
                let mut buf = ArrayVec::<u8, 33>::new_const();

                buf.push(FrameTag::ContentRequest as u8);
                buf.write_all(&hash.0).unwrap();

                self.write_stream.write_all(&buf).await?;
            }
            UrsaFrame::ContentResponse {
                compression,
                proof_len,
                block_len,
                signature,
            } => {
                let mut buf = ArrayVec::<u8, 82>::new_const();

                buf.push(FrameTag::ContentResponse as u8);
                buf.push(compression);
                buf.write_all(&proof_len.to_be_bytes()).unwrap();
                buf.write_all(&block_len.to_be_bytes()).unwrap();
                buf.write_all(&signature).unwrap();

                self.write_stream.write_all(&buf).await?;
            }
            UrsaFrame::ContentRangeRequest {
                hash,
                chunk_start,
                chunks,
            } => {
                let mut buf = ArrayVec::<u8, 43>::new_const();

                buf.push(FrameTag::ContentRangeRequest as u8);
                buf.write_all(&hash.0).unwrap();
                buf.write_all(&chunk_start.to_be_bytes()).unwrap();
                buf.write_all(&chunks.to_be_bytes()).unwrap();

                self.write_stream.write_all(&buf).await?;
            }
            UrsaFrame::UpdateEpochSignal(nonce) => {
                let mut buf = ArrayVec::<u8, 9>::new_const();

                buf.push(FrameTag::UpdateEpochSignal as u8);
                buf.write_all(&nonce.to_be_bytes()).unwrap();

                self.write_stream.write_all(&buf).await?;
            }
            UrsaFrame::DecryptionKeyRequest {
                delivery_acknowledgment,
            } => {
                let mut buf = ArrayVec::<u8, 97>::new_const();

                buf.push(FrameTag::DecryptionKeyRequest as u8);
                buf.write_all(&delivery_acknowledgment).unwrap();

                self.write_stream.write_all(&buf).await?;
            }
            UrsaFrame::DecryptionKeyResponse { decryption_key } => {
                let mut buf = ArrayVec::<u8, 34>::new_const();

                buf.push(FrameTag::DecryptionKeyResponse as u8);
                buf.write_all(&decryption_key).unwrap();

                self.write_stream.write_all(&buf).await?;
            }
            UrsaFrame::EndOfRequestSignal => {
                self.write_stream
                    .write_u8(FrameTag::EndOfRequestSignal as u8)
                    .await?
            }
            UrsaFrame::TerminationSignal(reason) => {
                let mut buf = ArrayVec::<u8, 2>::new_const();

                buf.push(FrameTag::TerminationSignal as u8);
                buf.push(reason as u8);

                self.write_stream.write_all(&buf).await?;
            }
        }

        Ok(())
    }

    /// Write a termination signal to the stream.
    #[inline(always)]
    pub async fn termination_signal(&mut self, reason: Option<Reason>) -> std::io::Result<()> {
        self.write_frame(UrsaFrame::TerminationSignal(
            reason.unwrap_or(Reason::Unknown),
        ))
        .await
    }
}

impl<R, W> UfdpConnection<R, W>
where
    R: AsyncRead + Unpin + Send + Sync,
    W: AsyncWrite + Unpin + Send + Sync,
{
    pub fn new(
        read_stream: impl Into<UfdpConnectionReadHalf<R>>,
        write_stream: impl Into<UfdpConnectionWriteHalf<W>>,
    ) -> Self {
        Self {
            read_half: read_stream.into(),
            write_half: write_stream.into(),
        }
    }

    #[inline(always)]
    pub fn read_buffer(&mut self, len: usize) {
        self.read_half.read_buffer(len)
    }

    #[inline(always)]
    pub async fn read_frame(&mut self, filter: Option<u8>) -> std::io::Result<Option<UrsaFrame>> {
        self.read_half.read_frame(filter).await
    }

    #[inline(always)]
    pub async fn write_frame(&mut self, frame: UrsaFrame) -> std::io::Result<()> {
        self.write_half.write_frame(frame).await
    }

    #[inline(always)]
    pub async fn termination_signal(&mut self, reason: Option<Reason>) -> std::io::Result<()> {
        self.write_half.termination_signal(reason).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tokio::{
        net::{TcpListener, TcpStream},
        sync::mpsc::channel,
    };

    type TResult = Result<(), UrsaCodecError>;

    async fn encode_decode(frame: UrsaFrame) -> TResult {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        // accept a single connection
        let (tx, mut rx) = channel(1);
        tokio::task::spawn(async move {
            let (s, _) = listener.accept().await.unwrap();
            tx.send(s).await.unwrap();
        });

        // create streams
        let alice_stream = TcpStream::connect(addr).await?;
        let bob_stream = rx.recv().await.unwrap();

        let (alice_r, alice_w) = alice_stream.split();
        let (bob_r, bob_w) = bob_stream.split();

        // create a raw ufdp connection to encode/decode with
        let mut alice = UfdpConnection::new(alice_r, alice_w);
        let mut bob = UfdpConnection::new(bob_r, bob_w);

        // write/read the frame, comparing the result afterwards
        alice.write_frame(frame.clone()).await?;
        let recv_frame = bob.read_frame(None).await?.unwrap();
        assert_eq!(frame, recv_frame);

        Ok(())
    }

    #[tokio::test]
    async fn handshake_req() -> TResult {
        encode_decode(UrsaFrame::HandshakeRequest {
            version: 0,
            supported_compression_bitmap: 0,
            lane: None,
            pubkey: [1u8; 48],
        })
        .await
    }

    #[tokio::test]
    async fn handshake_res() -> TResult {
        encode_decode(UrsaFrame::HandshakeResponse {
            lane: 0,
            epoch_nonce: 1000,
            pubkey: [1; 33],
            last: None,
        })
        .await?;

        encode_decode(UrsaFrame::HandshakeResponse {
            lane: 0,
            epoch_nonce: 1000,
            pubkey: [2; 33],
            last: Some(LastLaneData {
                bytes: 64,
                signature: [3; 96],
            }),
        })
        .await
    }

    #[tokio::test]
    async fn content_req() -> TResult {
        encode_decode(UrsaFrame::ContentRequest {
            hash: Blake3Cid([1; 32]),
        })
        .await
    }

    #[tokio::test]
    async fn content_range_req() -> TResult {
        encode_decode(UrsaFrame::ContentRangeRequest {
            hash: Blake3Cid([0u8; 32]),
            chunk_start: 1u64,
            chunks: 2u16,
        })
        .await
    }

    #[tokio::test]
    async fn content_res() -> TResult {
        // frame header decode
        encode_decode(UrsaFrame::ContentResponse {
            compression: 0,
            signature: [1u8; 64],
            proof_len: 64,
            block_len: 64,
        })
        .await
    }

    #[tokio::test]
    async fn decryption_key_req() -> TResult {
        encode_decode(UrsaFrame::DecryptionKeyRequest {
            delivery_acknowledgment: [1; 96],
        })
        .await
    }

    #[tokio::test]
    async fn decryption_key_res() -> TResult {
        encode_decode(UrsaFrame::DecryptionKeyResponse {
            decryption_key: [1; 33],
        })
        .await
    }

    #[tokio::test]
    async fn update_epoch_signal() -> TResult {
        encode_decode(UrsaFrame::UpdateEpochSignal(1024)).await
    }

    #[tokio::test]
    async fn termination_signal() -> TResult {
        encode_decode(UrsaFrame::TerminationSignal(Reason::Unknown)).await
    }
}
