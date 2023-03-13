use elliptic_curve::group::GroupEncoding;
use elliptic_curve::hash2curve::{hash_to_field, ExpandMsgXmd, MapToCurve};
use elliptic_curve::sec1::ToEncodedPoint;
use elliptic_curve::Field;
use rand::Rng;
use rand_core::RngCore;
use rand_core::{block::BlockRngCore, OsRng, SeedableRng};
use sha2::Sha256;
use std::iter::zip;

// TODO(qti3e): Remove the use of hash_to_curve that is using ExpandMsgXmd.
// and avoid hashing the same information multiple times and reuse the hashes.

// TODO(qti3e) Import this from somewhere else, this doesn't belong here.
pub type CID = blake3::Hash;

/// The request that
pub struct RequestInfo {
    /// The content identifier for the file that was requested.
    pub cid: CID,
    pub client: [u8; 32],
    pub time: u64,
    pub from_bytes: u64,
    pub to_bytes: u64,
}

impl RequestInfo {
    pub fn hash_to_curve(&self) -> k256::AffinePoint {
        let mut u = [k256::FieldElement::default(), k256::FieldElement::default()];

        hash_to_field::<ExpandMsgXmd<Sha256>, k256::FieldElement>(
            &[],
            &[b"fleek-network-pod-request"],
            &mut u,
        )
        .unwrap();

        let q0 = u[0].map_to_curve();
        let q1 = u[1].map_to_curve();

        (q0 + q1).to_affine()
    }

    pub fn hash(&self) -> blake3::Hash {
        let mut hasher = blake3::Hasher::new_derive_key("FLEEK_NETWORK_POD_REQUEST_HASH");
        hasher.update(self.cid.as_bytes());
        hasher.update(&self.client);
        hasher.update(&self.time.to_be_bytes());
        hasher.update(&self.from_bytes.to_be_bytes());
        hasher.update(&self.to_bytes.to_be_bytes());
        hasher.finalize()
    }

    pub fn rand(mut rng: impl RngCore) -> Self {
        let cid: [u8; 32] = rng.gen();

        Self {
            cid: blake3::Hash::from(cid),
            from_bytes: 0,
            to_bytes: 256 * 1024 * 1024,
            client: rng.gen(),
            time: rng.gen(),
        }
    }
}

pub struct SecretKey(pub k256::Scalar);

/// Encrypt a block of data and write the result to the provided mutable slice,
/// this function will return an additional 64 byte that should be send to
// TODO(qti3e) If possible in future make this function encrypt in place.
pub fn encrypt_block(
    secret: &SecretKey,
    req: &RequestInfo,
    buffer: &[u8],
    result: &mut [u8],
) -> [u8; 64] {
    assert_eq!(
        buffer.len(),
        result.len(),
        "plaintext and result buffer must be the same size."
    );

    // DO NOT CHANGE THIS UNLESS YOU MAKE SURE THE TRANSMUTE LOGIC IS ALSO WORKING.
    const BLOCK_SIZE: usize = 64;

    let request_hash = req.hash();
    let encryption_key = req.hash_to_curve() * secret.0;
    let encoded_point = encryption_key.to_affine().to_encoded_point(true);
    let seed = *blake3::hash(encoded_point.as_bytes()).as_bytes();

    let mut hc_block = [0u8; BLOCK_SIZE];
    let mut hc_rng = rand_hc::Hc128Core::from_seed(seed);

    let mut buffer_iter = buffer.chunks_exact(BLOCK_SIZE);
    let mut result_iter = result.chunks_exact_mut(BLOCK_SIZE);

    for (buffer_block, result_block) in zip(&mut buffer_iter, &mut result_iter) {
        unsafe {
            let u32x16: &mut [u32; 16] = std::mem::transmute(&mut hc_block);
            hc_rng.generate(u32x16);
        }

        // the compiler might not be able to reason about the bound checks
        // inside the following for loop and hence skip auto vectorization
        // this explicit constant size binding here makes sure the compiler
        // is aware of the bounds of `block` and can eliminate the bound checks
        // inside the loop.
        let buffer_block = &buffer_block[0..BLOCK_SIZE];
        let result_block = &mut result_block[0..BLOCK_SIZE];

        for i in 0..BLOCK_SIZE {
            result_block[i] = buffer_block[i] ^ hc_block[i];
        }
    }

    if buffer.len() % BLOCK_SIZE > 0 {
        unsafe {
            let u32x16: &mut [u32; 16] = std::mem::transmute(&mut hc_block);
            hc_rng.generate(u32x16);
        }

        let buffer_block = buffer_iter.remainder();
        let result_block = result_iter.into_remainder();

        for i in 0..buffer_block.len() {
            result_block[i] = buffer_block[i] ^ hc_block[i];
        }
    }

    let hash = {
        let mut hasher = blake3::Hasher::new();
        hasher.update_rayon(&result);
        hasher.finalize()
    };

    // schnorr commitment.

    let k = k256::Scalar::random(OsRng);
    let r = (k256::AffinePoint::GENERATOR * k).to_affine();
    let r_compressed = r.to_bytes().to_vec();

    let e = {
        let mut c = [k256::Scalar::ZERO];
        hash_to_field::<ExpandMsgXmd<Sha256>, k256::Scalar>(
            &[&r_compressed, hash.as_bytes(), request_hash.as_bytes()],
            &[b"fleek-network-pod"],
            &mut c,
        )
        .unwrap();
        c[0]
    };
    let s = k - secret.0 * e;
    let mut ret = [0; 64];
    ret[0..32].copy_from_slice(&e.to_bytes());
    ret[32..].copy_from_slice(&s.to_bytes());
    ret
}
