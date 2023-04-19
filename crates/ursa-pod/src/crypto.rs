use std::marker::PhantomData;

use arrayref::array_ref;

pub mod aes;
pub mod bls;
pub mod domain_separators;
pub mod ed25519;
pub mod key;
pub mod request;

/// A UFDP cryptography backend using `libsodium` binding for Ed25519, `openssl` binding for
/// the AES and `BLST` binding for BLS operations.
pub type OptimizedEngine = Engine<ed25519::libsodium_impl::Ed25519, aes::openssl_impl::Aes128Ctr>;

/// A generic UFDP cryptography backend implementing the routines using different backend.
pub struct Engine<Ed25519: ed25519::Ed25519Engine, Cipher: aes::CipherEngine> {
    ed25519: PhantomData<Ed25519>,
    cipher: PhantomData<Cipher>,
}

impl<Ed25519: ed25519::Ed25519Engine, Cipher: aes::CipherEngine> Engine<Ed25519, Cipher> {
    /// Encrypt a block of data provided by the `input` slice and write the result to the
    /// `output` slice.
    ///
    /// # Panics
    ///
    /// If the output buffer does not have sufficient capacity to write the encrypted data
    /// and the signature. It must be 64 bytes more than the input.
    pub fn encrypt_block(
        sk: &Ed25519::SecretKey,
        request_info: &request::RequestInfo,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<ed25519::SymmetricKey, Ed25519::Error> {
        assert_eq!(input.len() + 64, output.len());

        let request_info_hash = request_info.hash();
        let key = Ed25519::generate_symmetric_key(sk, &request_info_hash)?;

        let symmetric_key = key.hash();
        Cipher::apply_cipher(
            aes::Mode::Encrypt,
            symmetric_key,
            input,
            &mut output[..input.len()],
        );

        let ciphertext_hash = hash_ciphertext(&output[..input.len()]);
        let signature = Ed25519::sign_ciphertext(sk, &ciphertext_hash, &request_info_hash)?;
        output[input.len()..].copy_from_slice(&signature);

        Ok(key)
    }

    /// Perform the public verification on a ciphertext, this verifies the ciphertext tag
    /// against a node's public key and checks if the given ciphertext is signed-off by
    /// a certain node as the response for a certain request.
    pub fn verify_ciphertext(
        pk: &Ed25519::PublicKey,
        request_info: &request::RequestInfo,
        ciphertext: &[u8],
    ) -> Result<bool, Ed25519::Error> {
        if ciphertext.len() < 64 {
            return Ok(false);
        }

        let cipher_len = ciphertext.len() - 64;
        let request_info_hash = request_info.hash();
        let ciphertext_hash = hash_ciphertext(&ciphertext[..cipher_len]);
        let signature = array_ref![ciphertext, cipher_len, 64];

        Ed25519::verify_ciphertext(pk, &ciphertext_hash, &request_info_hash, signature)
    }

    /// Performs the decryption on a ciphertext writing the result to the buffer provided as
    /// plaintext, the plaintext buffer needs to be exactly 64 bytes less than the ciphertext.
    ///
    /// # Panics
    ///
    /// If the ciphertext is not exactly 64 bytes longer than the plaintext.
    pub fn decrypt_block(key: &ed25519::SymmetricKey, ciphertext: &[u8], plaintext: &mut [u8]) {
        assert_eq!(plaintext.len() + 64, ciphertext.len());
        let symmetric_key = key.hash();
        Cipher::apply_cipher(
            aes::Mode::Decrypt,
            symmetric_key,
            &ciphertext[0..plaintext.len()],
            plaintext,
        );
    }
}

/// Hash the ciphertext with Blake3 under the protocol specified DST.
#[inline]
pub fn hash_ciphertext(cipher: &[u8]) -> [u8; 32] {
    let len = cipher.len();
    if len >= 256 * 1024 || (len >= 128 * 1024 && len <= 138 * 1023) {
        // Use rayon on specific sizes that actually improves the performance.
        *blake3::Hasher::new_keyed(&domain_separators::CIPHERTEXT_DIGEST)
            .update_rayon(cipher)
            .finalize()
            .as_bytes()
    } else {
        *blake3::keyed_hash(&domain_separators::CIPHERTEXT_DIGEST, cipher).as_bytes()
    }
}
