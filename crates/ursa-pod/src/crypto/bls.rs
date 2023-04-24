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

fn hash_to_delivery_acknowledgment_message(
    _lane: u8,
    _session_nonce: &[u8; 32],
    _node: &[u8; 32],
    _bytes: u64,
) {
}

pub mod blst_impl {
    use super::*;
    use crate::crypto::key::FixedSizeEncoding;
    use blst::min_pk;

    pub struct Bls;

    impl SecretKey<32> for min_pk::SecretKey {
        type PublicKey = min_pk::PublicKey;

        type Error = ();

        fn generate() -> Result<Self, Self::Error> {
            todo!()
        }

        fn public_key(&self) -> Result<Self::PublicKey, Self::Error> {
            todo!()
        }
    }

    impl FixedSizeEncoding<32> for min_pk::SecretKey {
        fn try_from_bytes(_bytes: &[u8; 32]) -> Option<Self> {
            todo!()
        }

        fn to_bytes(&self) -> [u8; 32] {
            todo!()
        }
    }

    impl PublicKey<48> for min_pk::PublicKey {}

    impl FixedSizeEncoding<48> for min_pk::PublicKey {
        fn try_from_bytes(_bytes: &[u8; 48]) -> Option<Self> {
            todo!()
        }

        fn to_bytes(&self) -> [u8; 48] {
            todo!()
        }
    }

    impl BlsEngine for Bls {
        type SecretKey = min_pk::SecretKey;

        type PublicKey = min_pk::PublicKey;

        fn generate_delivery_acknowledgment(
            sk: &Self::SecretKey,
            _lane: u8,
            _session_nonce: &[u8; 32],
            _node: &impl PublicKey<32>,
            _bytes: u64,
        ) -> DeliveryAcknowledgment {
            todo!()
        }

        fn verify_delivery_acknowledgment(
            _pk: &Self::PublicKey,
            _lane: u8,
            _session_nonce: &[u8; 32],
            _node: &impl PublicKey<32>,
            _bytes: u64,
        ) -> DeliveryAcknowledgment {
            todo!()
        }
    }
}
