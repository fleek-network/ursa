use arrayref::array_ref;
use bytes::{BufMut, BytesMut};
use consts::*;
use tokio_util::codec::{Decoder, Encoder};

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
    /// Maximum bytes a proof can be. The maximum theoretical file we support is
    /// `2^64` bytes, given we transfer data as blocks of 256KiB (`2^18` bytes) the
    /// maximum number of chunks is `2^46`. So the maximum height of the hash tree
    /// will be 47. So we will have maximum of 47 hashes (hence `47 * 32`) and one byte
    /// per each 8 hash (`ceil(47 / 8) = 6`).
    pub const MAX_PROOF_SIZE: usize = 47 * 32 + 6;
    /// Maximum bytes a block can be
    pub const MAX_BLOCK_SIZE: usize = 256 * 1024;

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
    InsufficientBalance = 0x00,
    Unknown = 0xFF,
}

impl Reason {
    fn from_u8(byte: u8) -> Option<Self> {
        match byte {
            0x00 => Some(Self::InsufficientBalance),
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// Not a frame. Buffer contains a chunk of bytes initiated after the `UrsaCodec::read_buffer` method has been called.
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

/// Ursa Fair Delivery Codec for tokio's [`Encoder`] and [`Decoder`] traits.
#[derive(Default)]
pub struct UrsaCodec {
    take: usize,
    chunk_size: usize,
}

impl UrsaCodec {
    /// Instruct the codec to begin returning raw byte chunks (`UrsaFrame::Buffer`) until `size` is exhausted.
    #[inline(always)]
    pub fn read_buffer(&mut self, size: usize, chunk_size: usize) {
        self.take = size;
        self.chunk_size = chunk_size;
    }
}

impl Encoder<UrsaFrame> for UrsaCodec {
    type Error = UrsaCodecError;

    fn encode(&mut self, event: UrsaFrame, buf: &mut BytesMut) -> Result<(), Self::Error> {
        buf.reserve(event.size_hint());

        if let Some(tag) = event.tag() {
            buf.put_u8(tag as u8);
        }

        match event {
            UrsaFrame::HandshakeRequest {
                version,
                pubkey,
                supported_compression_bitmap,
                lane,
            } => {
                buf.put_slice(&NETWORK);
                buf.put_u8(version);
                buf.put_u8(supported_compression_bitmap);
                buf.put_u8(lane.unwrap_or(0xFF));
                buf.put_slice(&pubkey);
            }
            UrsaFrame::HandshakeResponse {
                pubkey,
                epoch_nonce,
                lane,
                last,
            } => {
                buf.put_u8(lane);
                buf.put_u64(epoch_nonce);
                buf.put_slice(&pubkey);

                match last {
                    None => buf.put_u8(0x00),
                    Some(data) => {
                        buf.reserve(104);
                        buf.put_u8(0x80);
                        buf.put_u64(data.bytes);
                        buf.put_slice(&data.signature);
                    }
                }
            }
            UrsaFrame::ContentRequest { hash } => {
                buf.put_slice(&hash);
            }
            UrsaFrame::ContentResponse {
                compression,
                proof_len,
                block_len,
                signature,
            } => {
                buf.put_u8(compression);
                buf.put_u64(proof_len);
                buf.put_u64(block_len);
                buf.put_slice(&signature);
            }
            UrsaFrame::ContentRangeRequest {
                hash,
                chunk_start,
                chunks,
            } => {
                buf.put_slice(&hash);
                buf.put_u64(chunk_start);
                buf.put_u16(chunks);
            }
            UrsaFrame::UpdateEpochSignal(nonce) => {
                buf.put_u64(nonce);
            }
            UrsaFrame::DecryptionKeyRequest {
                delivery_acknowledgment,
            } => {
                buf.put_slice(&delivery_acknowledgment);
            }
            UrsaFrame::DecryptionKeyResponse { decryption_key } => {
                buf.put_slice(&decryption_key);
            }
            UrsaFrame::EndOfRequestSignal => {}
            UrsaFrame::TerminationSignal(reason) => {
                buf.put_u8(reason as u8);
            }
            UrsaFrame::Buffer(bytes) => {
                buf.put(bytes);
            }
        }

        Ok(())
    }
}

impl Decoder for UrsaCodec {
    type Item = UrsaFrame;
    type Error = UrsaCodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let len = src.len();
        if len == 0 {
            return Ok(None);
        }

        // should we be reading a chunk right now?
        if self.take > 0 {
            let take = self.take.min(self.chunk_size);
            return Ok(if len >= take {
                self.take -= take;
                Some(UrsaFrame::Buffer(src.split_to(take)))
            } else {
                None
            });
        }

        // first frame byte is the tag
        let (size_hint, tag) = FrameTag::try_from(src[0]).map(|t| (t.size_hint(), t))?;
        if len < size_hint {
            return Ok(None);
        }
        match tag {
            FrameTag::HandshakeRequest => {
                let buf = src.split_to(size_hint);
                let network = &buf[1..5];
                if network != NETWORK {
                    return Err(UrsaCodecError::InvalidNetwork);
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
                let (buf, last) = match src[43] {
                    0x80 => {
                        let size_hint = size_hint + 104;
                        if len < size_hint {
                            return Ok(None);
                        }

                        let buf = src.split_to(size_hint);
                        let bytes_bytes = *array_ref!(buf, 44, 8);
                        let bytes = u64::from_be_bytes(bytes_bytes);
                        let signature = *array_ref!(buf, 52, 96);
                        (buf, Some(LastLaneData { bytes, signature }))
                    }
                    0x00 => (src.split_to(size_hint), None),
                    _ => return Err(UrsaCodecError::Unknown),
                };
                let lane = buf[1];
                let epoch_bytes = *array_ref!(buf, 2, 8);
                let epoch_nonce = u64::from_be_bytes(epoch_bytes);
                let pubkey = *array_ref!(buf, 10, 33);

                Ok(Some(UrsaFrame::HandshakeResponse {
                    pubkey,
                    epoch_nonce,
                    lane,
                    last,
                }))
            }
            FrameTag::ContentRequest => {
                let buf = src.split_to(size_hint);
                let hash = *array_ref!(buf, 1, 32);

                Ok(Some(UrsaFrame::ContentRequest { hash }))
            }
            FrameTag::ContentResponse => {
                let buf = src.split_to(size_hint);
                let compression = buf[1];
                let proof_len_bytes = *array_ref!(buf, 2, 8);
                let proof_len = u64::from_be_bytes(proof_len_bytes);
                let block_len_bytes = *array_ref!(buf, 10, 8);
                let block_len = u64::from_be_bytes(block_len_bytes);
                if block_len == 0 {
                    return Err(UrsaCodecError::ZeroLengthBlock);
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
                let buf = src.split_to(size_hint);
                let hash = *array_ref!(buf, 1, 32);
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
                let buf = src.split_to(size_hint);
                let delivery_acknowledgment = *array_ref!(buf, 1, 96);

                Ok(Some(UrsaFrame::DecryptionKeyRequest {
                    delivery_acknowledgment,
                }))
            }
            FrameTag::DecryptionKeyResponse => {
                let buf = src.split_to(size_hint);
                let decryption_key = *array_ref!(buf, 1, 33);

                Ok(Some(UrsaFrame::DecryptionKeyResponse { decryption_key }))
            }
            FrameTag::UpdateEpochSignal => {
                let buf = src.split_to(size_hint);
                let epoch_bytes = *array_ref!(buf, 1, 8);
                let epoch_nonce = u64::from_be_bytes(epoch_bytes);

                Ok(Some(UrsaFrame::UpdateEpochSignal(epoch_nonce)))
            }
            FrameTag::EndOfRequestSignal => Ok(Some(UrsaFrame::EndOfRequestSignal)),
            FrameTag::TerminationSignal => {
                let buf = src.split_to(size_hint);
                let byte = buf[1];

                if let Some(reason) = Reason::from_u8(byte) {
                    Ok(Some(UrsaFrame::TerminationSignal(reason)))
                } else {
                    Err(UrsaCodecError::InvalidReason(byte))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TResult = Result<(), UrsaCodecError>;

    fn run(encoded: &[u8], decoded: UrsaFrame) -> TResult {
        let mut codec = UrsaCodec::default();
        let mut buf = BytesMut::new();
        codec.encode(decoded.clone(), &mut buf)?;
        assert_eq!(buf, encoded);

        buf.clear();

        // simulate calling as bytes stream into the buffer
        for byte in encoded {
            buf.put_u8(*byte);
            if let Some(frame) = codec.decode(&mut buf)? {
                assert_eq!(frame, decoded);
                assert!(buf.is_empty());
            }
        }

        Ok(())
    }

    #[test]
    fn handshake_req() -> TResult {
        run(
            b"\x01URSA\0\0\xff\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01", 
            UrsaFrame::HandshakeRequest {
                version: 0,
                supported_compression_bitmap: 0,
                lane: None,
                pubkey: [1u8; 48],
            }
        )
    }

    #[test]
    fn handshake_res() -> TResult {
        run(
            b"\x81\0\0\0\0\0\0\0\x03\xe8\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\0",
            UrsaFrame::HandshakeResponse {
                lane: 0,
                epoch_nonce: 1000,
                pubkey: [1; 33],
                last: None,
            },
        )?;

        run(
            b"\x81\0\0\0\0\0\0\0\x03\xe8\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x80\0\0\0\0\0\0\0\x40\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03\x03",
            UrsaFrame::HandshakeResponse {
                lane: 0,
                epoch_nonce: 1000,
                pubkey: [2; 33],
                last: Some(LastLaneData {
                    bytes: 64,
                    signature: [3; 96],
                }),
            },
        )
    }

    #[test]
    fn content_req() -> TResult {
        run(
                b"\x02\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01",
                UrsaFrame::ContentRequest { hash: [1; 32] }
            )
    }

    #[test]
    fn content_range_req() -> TResult {
        run(
                b"\x04\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x01\0\x02",
                UrsaFrame::ContentRangeRequest {
                    hash: [0u8; 32],
                    chunk_start: 1u64,
                    chunks: 2u16,
                },
            )
    }

    #[test]
    fn content_res() -> TResult {
        // frame header decode
        run(
                b"\x82\0\0\0\0\0\0\0\0\x40\0\0\0\0\0\0\0\x40\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01",
                UrsaFrame::ContentResponse {
                    compression: 0,
                    signature: [1u8; 64],
                    proof_len: 64,
                    block_len: 64,
                },
            )?;

        // test frame chunk decode
        let mut codec = UrsaCodec::default();
        let mut buf = BytesMut::new();
        buf.put_slice(&[1u8; 64]); // proof
        buf.put_slice(&[2u8; 64]); // content

        // decode proof stream
        let mut count = 0;
        codec.read_buffer(64, 16);
        loop {
            match codec.decode(&mut buf) {
                Ok(Some(UrsaFrame::Buffer(data))) => {
                    count += 1;
                    assert_eq!(data, BytesMut::from([1u8; 16].as_slice()));
                }
                Ok(None) => continue,
                other => unreachable!("{other:?}"),
            }
            if count > 3 {
                break;
            }
        }

        // decode content stream
        count = 0;
        codec.read_buffer(64, 16);
        loop {
            match codec.decode(&mut buf) {
                Ok(Some(UrsaFrame::Buffer(data))) => {
                    count += 1;
                    assert_eq!(data, BytesMut::from([2u8; 16].as_slice()));
                }
                Ok(None) => continue,
                other => unreachable!("{other:?}"),
            }
            if count > 3 {
                break;
            }
        }

        Ok(())
    }

    #[test]
    fn decryption_key_req() -> TResult {
        run(
                b"\x08\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01",
                UrsaFrame::DecryptionKeyRequest {
                    delivery_acknowledgment: [1; 96],
                },
            )
    }

    #[test]
    fn decryption_key_res() -> TResult {
        run(
                b"\x88\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01",
                UrsaFrame::DecryptionKeyResponse {
                    decryption_key: [1; 33],
                },
            )
    }

    #[test]
    fn update_epoch_signal() -> TResult {
        run(
            b"\x90\0\0\0\0\0\0\x04\0",
            UrsaFrame::UpdateEpochSignal(1024),
        )
    }

    #[test]
    fn termination_signal() -> TResult {
        run(b"\xc0\xff", UrsaFrame::TerminationSignal(Reason::Unknown))
    }
}