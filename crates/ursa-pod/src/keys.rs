use elliptic_curve::Field;
use rand_core::RngCore;

/// The proof of delivery key of the node.
pub struct SecretKey {
    secret: k256::Scalar,
    /// The same key material but wrapped for the `secp256k1` library.
    secp256k1_key_pair: secp256k1::KeyPair,
}

pub struct PublicKey(pub k256::AffinePoint);

impl SecretKey {
    fn new_internal(secret: k256::Scalar) -> Self {
        let bytes = secret.to_bytes();
        let secp256k1_sk = secp256k1::SecretKey::from_slice(bytes.as_slice()).unwrap();
        let secp256k1_key_pair =
            secp256k1::KeyPair::from_secret_key(&*secp256k1::SECP256K1, &secp256k1_sk);

        Self {
            secret,
            secp256k1_key_pair,
        }
    }

    /// Create a new random secret key from the provided source of randomness.
    pub fn random(rng: impl RngCore) -> Self {
        Self::new_internal(k256::Scalar::random(rng))
    }

    /// Returns the public key of this secret key.
    pub fn public_key(&self) -> PublicKey {
        PublicKey((k256::AffinePoint::GENERATOR * self.secret).to_affine())
    }

    /// Returns the private key as an scalar value. Use this function cautiously and
    /// only when you know what you're doing.
    #[inline(always)]
    pub fn as_scalar(&self) -> &k256::Scalar {
        &self.secret
    }

    /// Returns the key pair for `secp256k1` library.
    #[inline(always)]
    pub fn as_secp256k1_key_pair(&self) -> &secp256k1::KeyPair {
        &self.secp256k1_key_pair
    }
}
