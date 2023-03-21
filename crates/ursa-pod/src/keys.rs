use elliptic_curve::Field;
use rand_core::RngCore;

/// The proof of delivery key of the node.
pub struct SecretKey {
    secret: k256::Scalar,
}

pub struct PublicKey(pub k256::AffinePoint);

impl SecretKey {
    /// Create a new random secret key from the provided source of randomness.
    pub fn random(rng: impl RngCore) -> Self {
        Self {
            secret: k256::Scalar::random(rng),
        }
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
}
