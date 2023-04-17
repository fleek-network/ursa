use arrayvec::ArrayVec;
use blake3::keyed_hash;
use rand::{Rng, RngCore};

use super::domain_separators;

/// The information about a singe block request.
pub struct RequestInfo {
    /// The root content id that was requested.
    pub cid: [u8; 32],
    /// The server's public key.
    pub server: [u8; 32],
    /// The client's public key.
    pub client: [u8; 48],
    /// Nonce assigned to the session.
    pub session_nonce: [u8; 32],
    /// Determines the block index which the user has requested.
    pub block_number: u64,
    /// The block counter in the entire session.
    pub block_counter: u64,
}

impl RequestInfo {
    /// Returns the hash of the request info.
    #[inline]
    pub fn hash(&self) -> [u8; 32] {
        let mut bytes = ArrayVec::<u8, { 32 + 32 + 48 + 32 + 8 + 8 }>::new();
        bytes.try_extend_from_slice(&self.cid).unwrap();
        bytes.try_extend_from_slice(&self.server).unwrap();
        bytes.try_extend_from_slice(&self.client).unwrap();
        bytes.try_extend_from_slice(&self.session_nonce).unwrap();
        bytes
            .try_extend_from_slice(&self.block_number.to_be_bytes())
            .unwrap();
        bytes
            .try_extend_from_slice(&self.block_counter.to_be_bytes())
            .unwrap();
        *keyed_hash(&domain_separators::HASH_REQUEST_INFO, &bytes).as_bytes()
    }

    /// Used for testing purposes to generate a random request info.
    pub fn rand(mut rng: impl RngCore) -> Self {
        Self {
            cid: rng.gen(),
            server: {
                let mut ret = [0; 32];
                rng.fill_bytes(&mut ret);
                ret
            },
            client: {
                let mut ret = [0; 48];
                rng.fill_bytes(&mut ret);
                ret
            },
            session_nonce: rng.gen(),
            block_number: rng.gen(),
            block_counter: rng.gen(),
        }
    }
}
