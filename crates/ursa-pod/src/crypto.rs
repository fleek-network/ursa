use crate::{
    keys::SecretKey,
    types::{Blake3Cid, BlsPublicKey, SchnorrSignature},
};
use arrayvec::ArrayVec;
use blake3::keyed_hash;
use elliptic_curve::{
    hash2curve::{FromOkm, MapToCurve},
    sec1::ToEncodedPoint,
};
use rand::Rng;
use rand_core::RngCore;

/// The information about a singe block request.
pub struct RequestInfo {
    /// The root content id that was requested.
    pub cid: Blake3Cid,
    /// The client's public key.
    pub client: BlsPublicKey,
    /// Nonce assigned to the session.
    pub session_nonce: [u8; 32],
    /// Determines the block index which the user has requested.
    pub block_counter: u64,
}

impl RequestInfo {
    /// Returns the hash of the request info.
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

    /// Used for testing purposes to generate a random request info.
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

/// Generate the encryption key that should be used for a request.
#[inline]
pub fn generate_symmetric_key(sk: &SecretKey, request_info_hash: &[u8; 32]) -> [u8; 32] {
    let request_info_on_curve = hash_to_curve(request_info_hash);
    let encryption_key = request_info_on_curve * sk.as_scalar();
    let encoded_point = encryption_key.to_affine().to_encoded_point(true);
    *keyed_hash(
        &ufdp_keys::HASH_TO_SYMMETRIC_KEY_KEY,
        encoded_point.as_bytes(),
    )
    .as_bytes()
}

/// The cipher's mode of operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Run the cipher to encrypt data.
    Encrypt,
    /// Run the cipher to decrypt data.
    Decrypt,
}

impl From<Mode> for openssl::symm::Mode {
    #[inline(always)]
    fn from(value: Mode) -> Self {
        match value {
            Mode::Encrypt => openssl::symm::Mode::Encrypt,
            Mode::Decrypt => openssl::symm::Mode::Decrypt,
        }
    }
}

/// Implementation of the `AES-128-CTR` using openssl.
// TODO(qti3e) we may want to provide a pure rust non-openssl implementation as well as a
// crate feature.
#[inline]
pub fn apply_aes_128_ctr(mode: Mode, key: [u8; 32], input: &[u8], output: &mut [u8]) {
    let mut encrypter = openssl::symm::Crypter::new(
        openssl::symm::Cipher::aes_128_ctr(),
        mode.into(),
        &key[0..16],
        Some(&key[16..]),
    )
    .unwrap();
    encrypter.update(input, output).unwrap();
}

/// Create a signature committing to the integrity of a ciphertext.
#[inline]
pub fn sign_ciphertext(
    sk: &SecretKey,
    ciphertext_hash: &[u8; 32],
    request_info_hash: &[u8; 32],
) -> SchnorrSignature {
    let hash = {
        let mut buffer = ArrayVec::<u8, { 32 + 32 }>::new();
        buffer.try_extend_from_slice(ciphertext_hash).unwrap();
        buffer.try_extend_from_slice(request_info_hash).unwrap();
        *keyed_hash(&ufdp_keys::SCHNORR_CHALLENGE_KEY, &buffer).as_bytes()
    };

    let msg = secp256k1::Message::from_slice(&hash).unwrap();
    *sk.as_secp256k1_key_pair().sign_schnorr(msg).as_ref()
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
