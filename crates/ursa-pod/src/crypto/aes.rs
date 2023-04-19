use std::fmt::Debug;

use zeroize::Zeroize;

/// The cipher's mode of operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Run the cipher to encrypt data.
    Encrypt,
    /// Run the cipher to decrypt data.
    Decrypt,
}

/// The key that should be used from
#[derive(PartialEq, PartialOrd, Zeroize)]
pub struct CipherKey(pub [u8; 32]);

pub trait CipherEngine {
    /// Apply an stream cipher to the input and write the resulting ciphertext to the
    /// buffer provided by `output`.
    fn apply_cipher(mode: Mode, key: CipherKey, input: &[u8], output: &mut [u8]);

    fn apply_cipher_in_place(mode: Mode, key: CipherKey, buffer: &mut [u8]) {
        unsafe {
            let len = buffer.len();
            let ptr = buffer.as_mut_ptr();
            let input = std::slice::from_raw_parts(ptr as *const u8, len);
            let output = std::slice::from_raw_parts_mut(ptr, len);
            Self::apply_cipher(mode, key, input, output)
        }
    }
}

impl Debug for CipherKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("CipherKey").finish()
    }
}

pub mod openssl_impl {
    use super::{CipherEngine, CipherKey, Mode};

    /// Implementer of the [`CipherEngine`] trait using the OpenSSL binding.
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
