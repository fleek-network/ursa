use crate::crypto::domain_separators;
use arrayref::array_ref;
use arrayvec::ArrayVec;
use blake3::keyed_hash;
use std::fmt::Debug;
use zeroize::Zeroize;

use super::aes::CipherKey;
use super::key::{PublicKey, SecretKey};

pub type Ed25519PointBytes = [u8; 32];
pub type Ed25519ScalarBytes = [u8; 32];

#[derive(Zeroize, PartialEq, Eq)]
pub struct SymmetricKey {
    /// The point `a * H(request_info_hash)`.
    pub point: Ed25519PointBytes,
    /// An Ed25519 signature from the node which is used to commit to
    /// this computation.
    pub signature: [u8; 64],
}

pub trait Ed25519Engine {
    /// The error used for this type.
    type Error: Debug;

    /// The secret key used by the implementation.
    type SecretKey: SecretKey<32, Error = Self::Error, PublicKey = Self::PublicKey>;

    /// The public key used by the implementation.
    type PublicKey: PublicKey<32>;

    /// Generate the symmetric key that should be used to encrypt a message. This includes the
    /// generated point and a zero-knowledge proof of DLE which proves the generated point is
    /// generated using the node's secret key.
    fn generate_symmetric_key(
        sk: &Self::SecretKey,
        request_info_hash: &[u8; 32],
    ) -> Result<SymmetricKey, Self::Error>;

    /// Run the public verification on the encryption key, this checks for the validity of the
    /// DLE zero-knowledge proof and returns the actual symmetric key that should be used for the
    /// cipher.
    ///
    /// The error should only be used if there was actually an error while performing the
    /// operation.
    fn verify_symmetric_key(
        pk: &Self::PublicKey,
        request_info_hash: &[u8; 32],
        key: &SymmetricKey,
    ) -> Result<Option<CipherKey>, Self::Error>;

    /// Sign a response for a request that can be used as public commitment
    /// to the returned ciphertext.
    fn sign_ciphertext(
        sk: &Self::SecretKey,
        ciphertext_hash: &[u8; 32],
        request_info_hash: &[u8; 32],
    ) -> Result<[u8; 64], Self::Error>;

    /// Verify the public commitment to a ciphertext originated by a node. Error is only
    /// returned if there is a failure from the backend implementation.
    fn verify_ciphertext(
        pk: &Self::PublicKey,
        ciphertext_hash: &[u8; 32],
        request_info_hash: &[u8; 32],
        signature: &[u8; 64],
    ) -> Result<bool, Self::Error>;
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

#[inline(always)]
fn hash_to_symmetric_key_commitment(point: &[u8; 32], request_info_hash: &[u8; 32]) -> [u8; 32] {
    let mut buffer = ArrayVec::<u8, { 32 + 32 }>::new();
    buffer.try_extend_from_slice(point).unwrap();
    buffer.try_extend_from_slice(request_info_hash).unwrap();
    *keyed_hash(&domain_separators::SYMMETRIC_KEY_COMMITMENT, &buffer).as_bytes()
}

impl SymmetricKey {
    /// Return the hash for this symmetric key, this should be fed to the cipher as the key.
    #[inline]
    pub fn hash(&self) -> CipherKey {
        CipherKey(*keyed_hash(&domain_separators::HASH_TO_SYMMETRIC_KEY, &self.point).as_bytes())
    }
}

impl From<SymmetricKey> for [u8; 96] {
    fn from(value: SymmetricKey) -> Self {
        let mut buffer = [0; 96];
        buffer[0..32].copy_from_slice(&value.point);
        buffer[32..].copy_from_slice(&value.signature);
        buffer
    }
}

impl From<[u8; 96]> for SymmetricKey {
    fn from(value: [u8; 96]) -> Self {
        Self {
            point: *array_ref![value, 0, 32],
            signature: *array_ref![value, 32, 64],
        }
    }
}

pub mod libsodium_impl {
    use super::*;
    use crate::crypto::key::{FixedSizeEncoding, SecretKey};
    use alkali::asymmetric::sign::{ed25519 as sign, SignError};
    use alkali::curve::ed25519 as curve;
    use alkali::AlkaliError;
    use zeroize::Zeroize;

    /// The Ed25519 backend for UFDP implemented using libsodium bindings. Instead of using
    /// the low level binding's the `alkali` crate is used for this implementation which
    /// provides Rust safe primitives and types on top of `libsodium-sys-stable`.
    pub struct Ed25519;

    #[derive(Zeroize)]
    pub struct Ed25519SecretKey {
        scalar: curve::Scalar<alkali::mem::FullAccess>,
        private_key: sign::PrivateKey<alkali::mem::FullAccess>,
    }

    pub struct Ed25519PublicKey(sign::PublicKey);

    impl TryFrom<sign::Keypair> for Ed25519SecretKey {
        type Error = AlkaliError;

        fn try_from(keypair: sign::Keypair) -> Result<Self, Self::Error> {
            // To get the scalar value that is used in the keypair, redo the same
            // steps as algorithm on the `seed` to derive to the result.
            let seed = keypair.get_seed()?;
            let secret = curve::Scalar::try_from(seed.as_ref())?;
            // This step performs the `SHA-512` on the seed.
            let secret = secret.to_curve25519()?;
            let scalar = curve::Scalar::try_from(secret.as_ref())?;

            // Check the logic and see if the scalar we computed can actually be used to
            // compute the public key.
            #[cfg(debug_assertions)]
            {
                let actual = curve::scalar_mult_base(&scalar)
                    .expect("Should not fail.")
                    .0;
                assert_eq!(actual, keypair.public_key);
            }

            Ok(Self {
                scalar,
                private_key: keypair.private_key,
            })
        }
    }

    impl SecretKey<32> for Ed25519SecretKey {
        type PublicKey = Ed25519PublicKey;
        type Error = AlkaliError;

        fn generate() -> Result<Self, AlkaliError> {
            let keypair = sign::Keypair::generate()?;
            Self::try_from(keypair)
        }

        fn public_key(&self) -> Result<Self::PublicKey, AlkaliError> {
            let keypair = sign::Keypair::from_private_key(&self.private_key)?;

            Ok(Ed25519PublicKey(keypair.public_key))
        }
    }

    impl FixedSizeEncoding<32> for Ed25519SecretKey {
        fn try_from_bytes(bytes: &[u8; 32]) -> Option<Self> {
            let seed = sign::Seed::try_from(bytes).ok()?;
            let keypair = sign::Keypair::from_seed(&seed).ok()?;
            Self::try_from(keypair).ok()
        }

        fn to_bytes(&self) -> [u8; 32] {
            let keypair = sign::Keypair::from_private_key(&self.private_key).unwrap();
            let seed = keypair.get_seed().unwrap();
            *seed.as_ref()
        }
    }

    impl PublicKey<32> for Ed25519PublicKey {}

    impl FixedSizeEncoding<32> for Ed25519PublicKey {
        fn try_from_bytes(bytes: &[u8; 32]) -> Option<Self> {
            Some(Ed25519PublicKey(*bytes))
        }

        fn to_bytes(&self) -> [u8; 32] {
            self.0
        }
    }

    impl Ed25519Engine for Ed25519 {
        type Error = AlkaliError;
        type SecretKey = Ed25519SecretKey;
        type PublicKey = Ed25519PublicKey;

        fn generate_symmetric_key(
            sk: &Self::SecretKey,
            request_info_hash: &[u8; 32],
        ) -> Result<SymmetricKey, Self::Error> {
            let h = curve::Point::from_uniform(request_info_hash)?;
            let point = h.scalar_mult(&sk.scalar)?;

            let keypair = sign::Keypair::from_private_key(&sk.private_key)?;
            let message = hash_to_symmetric_key_commitment(&point.0, &request_info_hash);
            let sign = sign::sign_detached(&message, &keypair)?;

            Ok(SymmetricKey {
                point: point.0,
                signature: sign.0,
            })
        }

        fn verify_symmetric_key(
            pk: &Self::PublicKey,
            request_info_hash: &[u8; 32],
            key: &SymmetricKey,
        ) -> Result<Option<CipherKey>, Self::Error> {
            let message = hash_to_symmetric_key_commitment(&key.point, &request_info_hash);
            let signature = sign::Signature(key.signature);
            let result = sign::verify_detached(&message, &signature, &pk.0);

            match result {
                Err(AlkaliError::SignError(SignError::InvalidSignature)) => Ok(None),
                Ok(()) => Ok(Some(key.hash())),
                Err(e) => Err(e),
            }
        }

        fn sign_ciphertext(
            sk: &Self::SecretKey,
            ciphertext_hash: &[u8; 32],
            request_info_hash: &[u8; 32],
        ) -> Result<[u8; 64], Self::Error> {
            let keypair = sign::Keypair::from_private_key(&sk.private_key)?;
            let message = hash_to_integrity_message(ciphertext_hash, request_info_hash);
            let sign = sign::sign_detached(&message, &keypair)?;
            Ok(sign.0)
        }

        fn verify_ciphertext(
            pk: &Self::PublicKey,
            ciphertext_hash: &[u8; 32],
            request_info_hash: &[u8; 32],
            signature: &[u8; 64],
        ) -> Result<bool, Self::Error> {
            let message = hash_to_integrity_message(ciphertext_hash, request_info_hash);
            let signature = sign::Signature(*signature);
            let result = sign::verify_detached(&message, &signature, &pk.0);

            match result {
                Err(AlkaliError::SignError(SignError::InvalidSignature)) => Ok(false),
                Ok(()) => Ok(true),
                Err(e) => Err(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::key::FixedSizeEncoding;

    fn test_sign_ciphertext<E: Ed25519Engine>() {
        let sk =
            <E::SecretKey as SecretKey<32>>::generate().expect("Failed to generate secret key.");

        let pk = sk.public_key().expect("Failed to get to the public key.");

        let request_info_hash = [1; 32];
        let ciphertext_hash = [2; 32];
        let signature = E::sign_ciphertext(&sk, &ciphertext_hash, &request_info_hash)
            .expect("Failed to sign message.");

        assert_eq!(
            E::verify_ciphertext(&pk, &ciphertext_hash, &request_info_hash, &signature).unwrap(),
            true
        );

        assert_eq!(
            E::verify_ciphertext(&pk, &ciphertext_hash, &request_info_hash, &[0; 64]).unwrap(),
            false
        );

        assert_eq!(
            E::verify_ciphertext(
                &<E::PublicKey as FixedSizeEncoding<32>>::try_from_bytes(&[0; 32]).unwrap(),
                &ciphertext_hash,
                &request_info_hash,
                &signature
            )
            .unwrap(),
            false
        );

        assert_eq!(
            E::verify_ciphertext(
                &pk,
                // swap
                &request_info_hash,
                &ciphertext_hash,
                &signature
            )
            .unwrap(),
            false
        );

        assert_eq!(
            E::verify_ciphertext(&pk, &[0; 32], &request_info_hash, &signature).unwrap(),
            false
        );

        assert_eq!(
            E::verify_ciphertext(&pk, &ciphertext_hash, &[0; 32], &signature).unwrap(),
            false
        );
    }

    fn test_generate_symmetric_key<E: Ed25519Engine>() {
        let sk =
            <E::SecretKey as SecretKey<32>>::generate().expect("Failed to generate secret key.");

        let pk = sk.public_key().expect("Failed to get to the public key.");

        let request_info_hash = [1; 32];
        let key =
            E::generate_symmetric_key(&sk, &request_info_hash).expect("Failed to generate key");
        let hash = key.hash();

        assert_eq!(
            E::verify_symmetric_key(&pk, &request_info_hash, &key).expect("Must be OK"),
            Some(hash)
        );

        assert_eq!(
            E::verify_symmetric_key(&pk, &[0; 32], &key).expect("Must be OK"),
            None
        );

        assert_eq!(
            E::verify_symmetric_key(
                &pk,
                &request_info_hash,
                &SymmetricKey {
                    point: key.point,
                    signature: [0; 64]
                }
            )
            .expect("Must be OK"),
            None
        );

        assert_eq!(
            E::verify_symmetric_key(
                &pk,
                &request_info_hash,
                &SymmetricKey {
                    point: [0; 32],
                    signature: key.signature
                }
            )
            .expect("Must be OK"),
            None
        );
    }

    fn test_secret_key_export<E: Ed25519Engine>() {
        let sk1 =
            <E::SecretKey as SecretKey<32>>::generate().expect("Failed to generate secret key.");

        let sk2 =
            <E::SecretKey as SecretKey<32>>::generate().expect("Failed to generate secret key.");

        let pk1 = sk1
            .public_key()
            .expect("Failed to get the public key.")
            .to_bytes();

        let pk2 = sk2
            .public_key()
            .expect("Failed to get the public key.")
            .to_bytes();

        assert_ne!(pk1, pk2);

        let sk1_imported =
            <E::SecretKey as FixedSizeEncoding<32>>::try_from_bytes(&sk1.to_bytes()).unwrap();

        let sk2_imported =
            <E::SecretKey as FixedSizeEncoding<32>>::try_from_bytes(&sk2.to_bytes()).unwrap();

        let pk1_imported = sk1_imported
            .public_key()
            .expect("Failed to get the public key.")
            .to_bytes();

        let pk2_imported = sk2_imported
            .public_key()
            .expect("Failed to get the public key.")
            .to_bytes();

        assert_eq!(pk1, pk1_imported);
        assert_eq!(pk2, pk2_imported);
    }

    mod test_sodium {
        use super::*;

        #[test]
        fn sign_ciphertext() {
            test_sign_ciphertext::<libsodium_impl::Ed25519>();
        }

        #[test]
        fn generate_symmetric_key() {
            test_generate_symmetric_key::<libsodium_impl::Ed25519>();
        }

        #[test]
        fn secret_key_export() {
            test_secret_key_export::<libsodium_impl::Ed25519>();
        }
    }
}
