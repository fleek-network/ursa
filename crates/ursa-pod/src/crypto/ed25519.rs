use crate::crypto::domain_separators;
use crate::types::SchnorrSignature;
use arrayvec::ArrayVec;
use blake3::keyed_hash;

use super::aes::CipherKey;
use super::key::{PublicKey, SecretKey};

pub type Ed25519PointBytes = [u8; 32];
pub type Ed25519ScalarBytes = [u8; 32];

/// The symmetric key with a zero-knowledge proof to make it publicly provable
/// right now a DLE is used, but a switch to an Ed25519 signature is possible
/// if it proves to be more efficient, as far as the number of bytes is concerned
/// both method have an equal 64-byte overhead.
pub struct SymmetricKey {
    /// The point `a * H(request_info_hash)`.
    pub point: Ed25519PointBytes,
    /// DLE challenge.
    pub challenge: Ed25519ScalarBytes,
    /// DLE response for the challenge.
    pub response: Ed25519ScalarBytes,
}

pub trait Ed25519Engine {
    /// The secret key used by the implementation.
    type SecretKey: SecretKey<32>;

    /// The public key used by the implementation.
    type PublicKey: PublicKey<32>;

    /// Generate the symmetric key that should be used to encrypt a message. This includes the
    /// generated point and a zero-knowledge proof of DLE which proves the generated point is
    /// generated using the node's secret key.
    fn generate_symmetric_key(sk: &Self::SecretKey, request_info_hash: &[u8; 32]) -> SymmetricKey;

    /// Run the public verification on the encryption key, this checks for the validity of the
    /// DLE zero-knowledge proof and returns the actual symmetric key that should be used for the
    /// cipher.
    fn verify_symmetric_key(
        pk: &Self::PublicKey,
        request_info_hash: &[u8; 32],
        key: &SymmetricKey,
    ) -> Option<CipherKey>;

    /// Sign a response for a request that can be used as public commitment
    /// to the returned ciphertext.
    fn sign_ciphertext(
        sk: &Self::SecretKey,
        ciphertext_hash: &[u8; 32],
        request_info_hash: &[u8; 32],
    ) -> SchnorrSignature;

    /// Verify the public commitment to a ciphertext originated by a node.
    fn verify_ciphertext(
        pk: &Self::SecretKey,
        ciphertext_hash: &[u8; 32],
        request_info_hash: &[u8; 32],
        signature: &SchnorrSignature,
    ) -> bool;
}

/// Hash the ciphertext hash and request info hash to the message that should
/// be signed to commit to the ciphertext integrity.
#[inline(always)]
fn hash_to_integrity_message(ciphertext_hash: &[u8; 32], request_info_hash: &[u8; 32]) -> [u8; 32] {
    let mut buffer = ArrayVec::<u8, { 32 + 32 }>::new();
    buffer.try_extend_from_slice(ciphertext_hash).unwrap();
    buffer.try_extend_from_slice(request_info_hash).unwrap();
    *keyed_hash(&domain_separators::CIPHERTEXT_COMMITMENT, &buffer).as_bytes()
}

impl SymmetricKey {
    /// Return the hash for this symmetric key, this should be fed to the cipher as the key.
    #[inline]
    fn hash(&self) -> CipherKey {
        CipherKey(*keyed_hash(&domain_separators::HASH_TO_SYMMETRIC_KEY, &self.point).as_bytes())
    }
}
