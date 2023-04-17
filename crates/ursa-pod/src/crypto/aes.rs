/// The cipher's mode of operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Run the cipher to encrypt data.
    Encrypt,
    /// Run the cipher to decrypt data.
    Decrypt,
}

/// The key that should be used from
pub struct CipherKey(pub [u8; 32]);

pub trait CipherEngine {
    /// Apply an stream cipher to the input and write the resulting ciphertext to the
    /// buffer provided by `output`.
    fn apply_cipher(mode: Mode, key: CipherKey, input: &[u8], output: &mut [u8]);
}

mod openssl_impl {
    use super::{CipherEngine, CipherKey, Mode};

    /// Implementer of the [`CipherEngine`] trait using the OpenSSL backend.
    pub struct Aes128Ctr;

    impl CipherEngine for Aes128Ctr {
        #[inline(always)]
        fn apply_cipher(mode: Mode, key: CipherKey, input: &[u8], output: &mut [u8]) {
            let mut encrypter = openssl::symm::Crypter::new(
                openssl::symm::Cipher::aes_128_ctr(),
                mode.into(),
                &key.0[0..16],
                Some(&key.0[16..]),
            )
            .unwrap();
            encrypter.update(input, output).unwrap();
        }
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
}
