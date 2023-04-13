use crate::{
    keys::SecretKey,
    types::{Blake3Cid, BlsPublicKey, SchnorrSignature},
};
use arrayref::array_ref;
use arrayvec::ArrayVec;
use blake3::keyed_hash;
use elliptic_curve::{
    hash2curve::{FromOkm, MapToCurve},
    sec1::ToEncodedPoint,
    Field,
};
use rand::Rng;
use rand_core::{block::BlockRngCore, OsRng, RngCore, SeedableRng};

pub struct RequestInfo {
    pub cid: Blake3Cid,
    pub client: BlsPublicKey,
    pub session_nonce: [u8; 32],
    pub block_counter: u64,
}

impl RequestInfo {
    #[inline]
    pub fn hash(&self) -> [u8; 32] {
        let mut bytes = ArrayVec::<u8, { 32 + 48 + 32 + 8 }>::new();
        bytes.try_extend_from_slice(&self.cid).unwrap();
        bytes.try_extend_from_slice(&self.client).unwrap();
        bytes.try_extend_from_slice(&self.session_nonce).unwrap();
        bytes
            .try_extend_from_slice(&self.block_counter.to_be_bytes())
            .unwrap();
        *keyed_hash(&ufdp_keys::HASH_REQUEST_INFO_KEY, &bytes).as_bytes()
    }

    pub fn rand(mut rng: impl RngCore) -> Self {
        Self {
            cid: rng.gen(),
            client: {
                let mut ret = [0; 48];
                rng.fill_bytes(&mut ret);
                ret
            },
            session_nonce: rng.gen(),
            block_counter: rng.gen(),
        }
    }
}

/// Hash the given data to a point on the elliptic curve.
#[inline]
pub fn hash_to_curve(input: &[u8]) -> k256::ProjectivePoint {
    let mut expander = blake3::Hasher::new_keyed(&ufdp_keys::HASH_TO_FIELD_KEY)
        .update(input)
        .finalize_xof();

    let mut buffer = [0; 48];

    expander.fill(&mut buffer);
    let q0 = k256::FieldElement::from_okm(&buffer.into()).map_to_curve();

    expander.fill(&mut buffer);
    let q1 = k256::FieldElement::from_okm(&buffer.into()).map_to_curve();

    q0 + q1
}

#[inline]
pub fn generate_encryption_key(sk: &SecretKey, request_info_hash: &[u8; 32]) -> [u8; 32] {
    let request_info_on_curve = hash_to_curve(request_info_hash);
    let encryption_key = request_info_on_curve * sk.as_scalar();
    let encoded_point = encryption_key.to_affine().to_encoded_point(true);
    *keyed_hash(
        &ufdp_keys::HASH_TO_SYMMETRIC_KEY_KEY,
        encoded_point.as_bytes(),
    )
    .as_bytes()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Encrypt,
    Decrypt,
}

impl From<Mode> for openssl::symm::Mode {
    fn from(value: Mode) -> Self {
        match value {
            Mode::Encrypt => openssl::symm::Mode::Encrypt,
            Mode::Decrypt => openssl::symm::Mode::Decrypt,
        }
    }
}

pub fn apply_cipher_in_place(key: [u8; 32], buffer: &mut [u8]) {
    let mut encrypter = openssl::symm::Crypter::new(
        openssl::symm::Cipher::aes_128_ctr(),
        openssl::symm::Mode::Encrypt,
        &key[0..16],
        Some(&key[16..]),
    )
    .unwrap();

    unsafe {
        let len = buffer.len();
        let ptr = buffer.as_ptr();
        let input = std::slice::from_raw_parts(ptr, len);
        let output = std::slice::from_raw_parts_mut(buffer.as_mut_ptr(), len);

        encrypter.update(input, output).unwrap();
    }
}

pub fn sign_response(
    kp: &secp256k1::KeyPair,
    ciphertext_hash: &[u8; 32],
    request_info_hash: &[u8; 32],
) -> SchnorrSignature {
    let hash = {
        let mut buffer = ArrayVec::<u8, { 32 + 32 }>::new();
        buffer.try_extend_from_slice(ciphertext_hash).unwrap();
        buffer.try_extend_from_slice(request_info_hash).unwrap();
        *blake3::keyed_hash(&ufdp_keys::SCHNORR_CHALLENGE_KEY, &buffer).as_bytes()
    };

    let msg = secp256k1::Message::from_slice(&hash).unwrap();
    *kp.sign_schnorr(msg).as_ref()
}

pub fn encrypt_in_place(sk: &SecretKey, req: &RequestInfo, buffer: &mut [u8]) -> SchnorrSignature {
    todo!()
}

/// The pre-computed protocol specific unique domain separators.
pub mod ufdp_keys {
    use hex_literal::hex;

    /// Should be used to compress a request info raw bytes.
    pub const HASH_REQUEST_INFO_KEY: [u8; 32] =
        hex!("4D85E693C2204AE36F69DE8664498AEFF5CA26DD350D9D01C81D818F589C3C8E");

    /// Used for when we hash things to the field element.
    pub const HASH_TO_FIELD_KEY: [u8; 32] =
        hex!("8A4F67FA3FFF7BB0D0226F0E960A79691263D9DA1F340BA0DFEDEF6CB969AC6C");

    /// Used for when we hash things to the field element.
    pub const HASH_TO_SYMMETRIC_KEY_KEY: [u8; 32] =
        hex!("F9C8329F93E84FFE57AB9963D86B1F8369665FB741381671AF8B335C9F0907DA");

    /// Used for when we hash things to the field element.
    pub const SCHNORR_CHALLENGE_KEY: [u8; 32] =
        hex!("7d169ac59f0c512273d77859f0349c9efedc1524f83851ae0f06fa2d04b0b73e");

    #[cfg(test)]
    mod tests {
        use super::*;
        use blake3::derive_key;

        #[test]
        fn hash_request_info_key() {
            let key = derive_key("HASH_REQUEST_INFO", b"FLEEK-NETWORK-UFDP");
            assert_eq!(
                key,
                HASH_REQUEST_INFO_KEY,
                "expected='{}'",
                blake3::Hash::from(key).to_hex()
            );
        }

        #[test]
        fn hash_to_field_key() {
            let key = derive_key("HASH_TO_FIELD_XOF", b"FLEEK-NETWORK-UFDP");
            assert_eq!(
                key,
                HASH_TO_FIELD_KEY,
                "expected='{}'",
                blake3::Hash::from(key).to_hex()
            );
        }

        #[test]
        fn hash_to_symmetric_key_key() {
            let key = derive_key("HASH_TO_SYMMETRIC_KEY", b"FLEEK-NETWORK-UFDP");
            assert_eq!(
                key,
                HASH_TO_SYMMETRIC_KEY_KEY,
                "expected='{}'",
                blake3::Hash::from(key).to_hex()
            );
        }

        #[test]
        fn schnorr_challenge_key() {
            let key = derive_key("SCHNORR_CHALLENGE", b"FLEEK-NETWORK-UFDP");
            assert_eq!(
                key,
                SCHNORR_CHALLENGE_KEY,
                "expected='{}'",
                blake3::Hash::from(key).to_hex()
            );
        }
    }
}
