use arrayref::array_ref;
use bytes::{BufMut, Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

pub type EpochNonce = u64;
pub type Secp256k1AffinePoint = [u8; 33];

pub type Blake3CID = [u8; 32];

pub type Secp256k1PublicKey = Secp256k1AffinePoint;
pub type SchnorrSignature = [u8; 64];

pub type BLSPublicKey = [u8; 48];
pub type BLSSignature = [u8; 96];

pub const NETWORK: [u8; 4] = *b"URSA";
pub const MAX_FRAME_SIZE: usize = 1024;
pub const MAX_LANES: u8 = 24;

// The bit flag on any frame sent from the node to the client.
pub const IS_RES_FLAG: u8 = 0b10000000;

// Request and response tags.
pub const HANDSHAKE_REQ_TAG: u8 = 0x01 << 0;
pub const HANDSHAKE_RES_TAG: u8 = IS_RES_FLAG | HANDSHAKE_REQ_TAG;
pub const CONTENT_REQ_TAG: u8 = 0x01 << 1;
pub const CONTENT_RANGE_REQ_TAG: u8 = 0x01 << 2;
pub const CONTENT_RES_TAG: u8 = IS_RES_FLAG | CONTENT_REQ_TAG;
pub const DECRYPTION_KEY_REQ_TAG: u8 = 0x01 << 3;
pub const DECRYPTION_KEY_RES_TAG: u8 = IS_RES_FLAG | DECRYPTION_KEY_REQ_TAG;

// Signals sent from the node to the client, signals are not a response to a particular
// request, but they still have the `IS_RES` bit enabled since they are sent from the
// node to the client.
pub const UPDATE_EPOCH_SIGNAL_TAG: u8 = IS_RES_FLAG | (0x01 << 4);
pub const END_OF_REQUEST_SIGNAL_TAG: u8 = IS_RES_FLAG | (0x01 << 5);
pub const TERMINATATION_SIGNAL_TAG: u8 = IS_RES_FLAG | (0x01 << 6);

// Supported compression algorithm bitmap values.
pub const NONE: u8 = 0;
pub const SNAPPY: u8 = 1;
pub const GZIP: u8 = 1 << 2;
pub const LZ4: u8 = 1 << 3;

#[derive(Default)]
pub struct UrsaCodec {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blake3Tree {}

impl Blake3Tree {
    fn to_bytes(&self) -> Bytes {
        Bytes::from_static(b"blake3tree")
    }

    fn from_bytes(_bytes: &mut BytesMut) -> Self {
        Self {}
    }
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LastLaneData {}

/// Frame variants for different requests and responses
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UrsaFrame {
    /// [ TAG . b"URSA" . version (1) . supported compression algorithm bitmap (1) . session lane . pubkey (48) ]
    /// size: 56 bytes
    /// To let the node select the lane automatically, set the value to 0xFF
    HandshakeRequest {
        version: u8,
        supported_compression_bitmap: u8,
        lane: u8,
        pubkey: BLSPublicKey,
    },
    /// [ TAG . pubkey (32) . epoch nonce (8) . lane (1) . last (1?) ]
    /// size: 43 bytes
    HandshakeResponse {
        pubkey: Secp256k1PublicKey,
        epoch_nonce: EpochNonce,
        lane: u8,
        last: Option<LastLaneData>,
    },
    /// [ TAG . blake3hash (32) ]
    /// size: 33 bytes
    ContentRequest { hash: Blake3CID },
    /// [ TAG . compression (1) . signature (64) . proof length (8) . proof (..16384) . content length (8) . content (..262144) ]
    /// size: 82 + proof len (max 16KB) + content len (max 256KB)
    ContentResponse {
        compression: u8,
        signature: SchnorrSignature,
        proof: Blake3Tree,
        content: Bytes,
    },
    /// [ TAG . blake3hash (32) . u64 (8) . u16 (2) ]
    /// size: 43 bytes
    RangeRequest {
        hash: Blake3CID,
        chunk_start: u64,
        chunks: u16,
    },
    /// [ TAG . bls signature (96) ]
    /// size: 97 bytes
    DecryptionKeyRequest {
        delivery_acknowledgement: BLSSignature,
    },
    /// [ TAG . decryption key (33) ]
    /// size: 34 bytes
    DecryptionKeyResponse {
        decryption_key: Secp256k1AffinePoint,
    },
    /// [ TAG . epoch nonce (8) ]
    /// size: 9 bytes
    UpdateEpochSignal(EpochNonce),
    /// [ TAG ]
    /// size: 1 byte
    EndOfRequestSignal,
    /// [ TAG . reason (1) ]
    /// size: 2 bytes
    TerminationSignal(Reason),
}

impl UrsaFrame {
    #[inline(always)]
    pub fn tag(&self) -> u8 {
        match self {
            Self::HandshakeRequest { .. } => HANDSHAKE_REQ_TAG,
            Self::HandshakeResponse { .. } => HANDSHAKE_RES_TAG,
            Self::ContentRequest { .. } => CONTENT_REQ_TAG,
            Self::RangeRequest { .. } => CONTENT_RANGE_REQ_TAG,
            Self::ContentResponse { .. } => CONTENT_RES_TAG,
            Self::DecryptionKeyRequest { .. } => DECRYPTION_KEY_REQ_TAG,
            Self::DecryptionKeyResponse { .. } => DECRYPTION_KEY_RES_TAG,
            Self::UpdateEpochSignal(_) => UPDATE_EPOCH_SIGNAL_TAG,
            Self::EndOfRequestSignal => END_OF_REQUEST_SIGNAL_TAG,
            Self::TerminationSignal(_) => TERMINATATION_SIGNAL_TAG,
        }
    }
}

#[derive(Debug)]
pub enum UrsaCodecError {
    InvalidNetwork,
    InvalidTag(u8),
    InvalidReason(u8),
    Io(std::io::Error),
    Unknown,
}

impl From<std::io::Error> for UrsaCodecError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl Encoder<UrsaFrame> for UrsaCodec {
    type Error = UrsaCodecError;

    fn encode(&mut self, event: UrsaFrame, buf: &mut BytesMut) -> Result<(), Self::Error> {
        buf.put_u8(event.tag());
        match event {
            UrsaFrame::HandshakeRequest {
                version,
                pubkey,
                supported_compression_bitmap,
                lane,
            } => {
                buf.reserve(55);
                buf.put_slice(&NETWORK);
                buf.put_u8(version);
                buf.put_u8(supported_compression_bitmap);
                buf.put_u8(lane);
                buf.put_slice(&pubkey);
            }
            UrsaFrame::HandshakeResponse {
                pubkey,
                epoch_nonce,
                lane,
                last,
            } => {
                let last = match last {
                    None => [0x00].as_slice(),
                    Some(_data) => [0x80].as_slice(),
                };
                buf.reserve(42 + last.len());
                buf.put_u8(lane);
                buf.put_u64(epoch_nonce);
                buf.put_slice(&pubkey);
                buf.put_slice(last);
            }
            UrsaFrame::ContentRequest { hash } => {
                buf.put_slice(&hash);
            }
            UrsaFrame::ContentResponse {
                proof,
                compression,
                signature,
                content,
            } => {
                let proof = proof.to_bytes();
                let proof_len = proof.len();
                let content_len = content.len();
                buf.reserve(67 + proof_len + content_len);
                buf.put_u8(compression);
                buf.put_slice(&signature);
                buf.put_u64(proof_len as u64);
                buf.put(proof);
                buf.put_u64(content_len as u64);
                buf.put_slice(&content);
            }
            UrsaFrame::RangeRequest {
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
                delivery_acknowledgement,
            } => {
                buf.put_slice(&delivery_acknowledgement);
            }
            UrsaFrame::DecryptionKeyResponse { decryption_key } => {
                buf.put_slice(&decryption_key);
            }
            UrsaFrame::EndOfRequestSignal => {}
            UrsaFrame::TerminationSignal(reason) => {
                buf.put_u8(reason as u8);
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

        // first frame byte is the tag
        let tag = src[0];
        match tag {
            HANDSHAKE_REQ_TAG => {
                if len < 56 {
                    return Ok(None);
                }

                let buf = src.split_to(56);
                let network = &buf[1..5];
                if network != NETWORK {
                    return Err(UrsaCodecError::InvalidNetwork);
                }

                let version = buf[5];
                let supported_compression_bitmap = buf[6];
                let lane = buf[7];
                let mut pubkey = [0u8; 48];
                pubkey.copy_from_slice(&buf[8..]);

                Ok(Some(UrsaFrame::HandshakeRequest {
                    version,
                    pubkey,
                    supported_compression_bitmap,
                    lane,
                }))
            }
            HANDSHAKE_RES_TAG => {
                if len < 44 {
                    return Ok(None);
                }

                let buf = src.split_to(44);
                let lane = buf[1];
                let mut epoch_bytes = [0u8; 8];
                epoch_bytes.copy_from_slice(&buf[2..10]);
                let epoch_nonce = u64::from_be_bytes(epoch_bytes);
                let mut pubkey = [0u8; 33];
                pubkey.copy_from_slice(&buf[10..43]);
                let last = match buf[43] {
                    0x80 => Some(LastLaneData {}),
                    0x00 => None,
                    _ => return Err(UrsaCodecError::Unknown),
                };

                Ok(Some(UrsaFrame::HandshakeResponse {
                    pubkey,
                    epoch_nonce,
                    lane,
                    last,
                }))
            }
            CONTENT_REQ_TAG => {
                if len < 33 {
                    return Ok(None);
                }

                let buf = src.split_to(33);
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&buf[1..33]);

                Ok(Some(UrsaFrame::ContentRequest { hash }))
            }
            CONTENT_RANGE_REQ_TAG => {
                if len < 43 {
                    return Ok(None);
                }

                let buf = src.split_to(43);
                let hash = *array_ref!(buf, 1, 32);
                let chunk_start_bytes = *array_ref!(buf, 33, 8);
                let chunk_start = u64::from_be_bytes(chunk_start_bytes);
                let chunks = u16::from_be_bytes([buf[41], buf[42]]);
                Ok(Some(UrsaFrame::RangeRequest {
                    hash,
                    chunk_start,
                    chunks,
                }))
            }
            CONTENT_RES_TAG => {
                if len < 74 {
                    return Ok(None);
                }

                let proof_len_bytes = *array_ref!(src, 66, 8);
                let proof_len = u64::from_be_bytes(proof_len_bytes) as usize;

                if len < 82 + proof_len {
                    return Ok(None);
                }

                let content_len_bytes = *array_ref!(src, 74, 8);
                let content_len = u64::from_be_bytes(content_len_bytes) as usize;

                let len = src.len();
                if len < 90 + proof_len + content_len {
                    return Ok(None);
                }

                let compression = src[1];
                let signature = *array_ref!(src, 2, 64);

                let _ = src.split_to(82);
                let mut proof_bytes = src.split_to(proof_len);
                let proof = Blake3Tree::from_bytes(&mut proof_bytes);

                let _ = src.split_to(8);
                let content = src.split_to(content_len).freeze();

                Ok(Some(UrsaFrame::ContentResponse {
                    compression,
                    signature,
                    proof,
                    content,
                }))
            }
            DECRYPTION_KEY_REQ_TAG => {
                if len < 97 {
                    return Ok(None);
                }

                let buf = src.split_to(97);
                let delivery_acknowledgement = *array_ref!(buf, 1, 96);

                Ok(Some(UrsaFrame::DecryptionKeyRequest {
                    delivery_acknowledgement,
                }))
            }
            DECRYPTION_KEY_RES_TAG => {
                if len < 34 {
                    return Ok(None);
                }

                let buf = src.split_to(34);
                let decryption_key = *array_ref!(buf, 1, 33);

                Ok(Some(UrsaFrame::DecryptionKeyResponse { decryption_key }))
            }
            UPDATE_EPOCH_SIGNAL_TAG => {
                if len < 9 {
                    return Ok(None);
                }

                let buf = src.split_to(9);
                let epoch_bytes = *array_ref!(buf, 1, 8);
                let epoch_nonce = u64::from_be_bytes(epoch_bytes);

                Ok(Some(UrsaFrame::UpdateEpochSignal(epoch_nonce)))
            }
            TERMINATATION_SIGNAL_TAG => {
                if len < 2 {
                    return Ok(None);
                }

                let buf = src.split_to(2);
                let byte = buf[1];

                if let Some(reason) = Reason::from_u8(byte) {
                    Ok(Some(UrsaFrame::TerminationSignal(reason)))
                } else {
                    Err(UrsaCodecError::InvalidReason(byte))
                }
            }
            t => Err(UrsaCodecError::InvalidTag(t)),
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

        // simulate calling as bytes stream into the buffer
        for byte in encoded {
            buf.put_u8(*byte);
            if let Some(frame) = codec.decode(&mut buf)? {
                assert_eq!(frame, decoded);
            }
        }

        Ok(())
    }

    #[test]
    fn handshake_req() -> TResult {
        let encoded = b"\0URSA\0\0\xff\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01";

        let decoded = UrsaFrame::HandshakeRequest {
            version: 0,
            supported_compression_bitmap: 0,
            lane: 0xFF,
            pubkey: [1u8; 48],
        };

        run(encoded, decoded)
    }

    #[test]
    fn handshake_res() -> TResult {
        run(
                b"\x80\0\0\0\0\0\0\0\x03\xe8\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\0",
                UrsaFrame::HandshakeResponse {
                    lane: 0,
                    epoch_nonce: 1000,
                    pubkey: [1; 33],
                    last: None,
                },
            )
    }

    #[test]
    fn content_req() -> TResult {
        run(
                b"\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01",
                UrsaFrame::ContentRequest { hash: [1; 32] }
            )
    }

    #[test]
    fn content_range_req() -> TResult {
        run(
                b"\x02\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x01\0\x02",
                UrsaFrame::RangeRequest {
                    hash: [0u8; 32],
                    chunk_start: 1u64,
                    chunks: 2u16,
                },
            )
    }

    #[test]
    fn content_res() -> TResult {
        run(
                b"\x81\0\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\0\0\0\0\0\0\0\x0ablake3tree\0\0\0\0\0\0\0\x20\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02\x02",
                UrsaFrame::ContentResponse {
                    compression: 0,
                    signature: [1u8; 64],
                    proof: Blake3Tree {},
                    content: Bytes::from([2u8; 32].as_slice()),
                },
            )
    }

    #[test]
    fn decryption_key_req() -> TResult {
        run(
                b"\x03\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01",
                UrsaFrame::DecryptionKeyRequest {
                    delivery_acknowledgement: [1; 96],
                },
            )
    }

    #[test]
    fn decryption_key_res() -> TResult {
        run(
                b"\x83\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01\x01",
                UrsaFrame::DecryptionKeyResponse {
                    decryption_key: [1; 33],
                },
            )
    }

    #[test]
    fn update_epoch_signal() -> TResult {
        run(
            b"\x82\0\0\0\0\0\0\x04\0",
            UrsaFrame::UpdateEpochSignal(1024),
        )
    }

    #[test]
    fn termination_signal() -> TResult {
        run(b"\xFF\xFF", UrsaFrame::TerminationSignal(Reason::Unknown))
    }
}
