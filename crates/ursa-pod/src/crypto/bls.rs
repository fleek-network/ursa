use super::key::{PublicKey, SecretKey};

pub struct DeliveryAcknowledgment {
    pub signature: [u8; 96],
}

pub trait BlsEngine {
    /// The secret key used by the implementation.
    type SecretKey: SecretKey<32>;

    /// The public key used by the implementation.
    type PublicKey: PublicKey<48>;

    /// Generate delivery acknowledgment for a given node.
    fn generate_delivery_acknowledgment(
        sk: &Self::SecretKey,
        lane: u8,
        session_nonce: &[u8; 32],
        node: &impl PublicKey<32>,
        bytes: u64,
    ) -> DeliveryAcknowledgment;

    /// Verify a single delivery acknowledgment.
    fn verify_delivery_acknowledgment(
        pk: &Self::PublicKey,
        lane: u8,
        session_nonce: &[u8; 32],
        node: &impl PublicKey<32>,
        bytes: u64,
    ) -> DeliveryAcknowledgment;
}
