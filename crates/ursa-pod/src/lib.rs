use elliptic_curve::hash2curve::{FromOkm, MapToCurve};
use elliptic_curve::sec1::ToEncodedPoint;
use elliptic_curve::Field;
use rand::Rng;
use rand_core::RngCore;
use rand_core::{block::BlockRngCore, OsRng, SeedableRng};
use std::iter::zip;

const REQUEST_INFO_HASH_DOMAIN_SEP: &'static str = "FLEEK_NETWORK_POD_REQUEST_HASH";
const SCHNORR_CHALLENGE_DOMAIN_SEP: &'static str = "FLEEK_NETWORK_POD_SCHNORR_CHALLENGE";

pub struct RequestInfo {
    pub cid: [u8; 32],
    pub client: [u8; 32],
    pub time: u64,
    pub from_bytes: u64,
    pub to_bytes: u64,
}

impl RequestInfo {
    #[inline(always)]
    fn hash_xof(&self) -> blake3::OutputReader {
        let mut hasher = blake3::Hasher::new_derive_key(REQUEST_INFO_HASH_DOMAIN_SEP);
        hasher.update(&self.cid);
        hasher.update(&self.client);
        hasher.update(&self.time.to_be_bytes());
        hasher.update(&self.from_bytes.to_be_bytes());
        hasher.update(&self.to_bytes.to_be_bytes());
        hasher.finalize_xof()
    }

    pub fn rand(mut rng: impl RngCore) -> Self {
        Self {
            cid: rng.gen(),
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

    // Use the same blake3 reader for computing the hash and the mapping to the curve.
    // this reader is stateful but has a cheap clone.
    let request_info_reader = req.hash_xof();

    let request_info_hash = {
        let mut buffer = [0; 32];
        request_info_reader.clone().fill(&mut buffer);
        buffer
    };

    let request_info_on_curve = {
        let mut buffer = [0; 48];
        let mut reader = request_info_reader;

        reader.fill(&mut buffer);
        let q0 = k256::FieldElement::from_okm(&buffer.into()).map_to_curve();

        reader.fill(&mut buffer);
        let q1 = k256::FieldElement::from_okm(&buffer.into()).map_to_curve();

        q0 + q1
    };

    let seed = {
        let encryption_key = request_info_on_curve * secret.0;
        let encoded_point = encryption_key.to_affine().to_encoded_point(true);
        blake3::hash(encoded_point.as_bytes())
    };

    let mut hc_block = [0u8; BLOCK_SIZE];
    let mut hc_rng = rand_hc::Hc128Core::from_seed(*seed.as_bytes());

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

    let encrypted_hash = {
        let mut hasher = blake3::Hasher::new();
        hasher.update_rayon(&result);
        hasher.finalize()
    };

    // schnorr commitment to the encrypted data.

    let k = k256::Scalar::random(OsRng);
    let r = (k256::AffinePoint::GENERATOR * k).to_affine();

    let e = {
        let mut okm: [u8; 48] = [0; 48];
        let mut hasher = blake3::Hasher::new_derive_key(SCHNORR_CHALLENGE_DOMAIN_SEP);
        hasher.update(r.to_encoded_point(false).as_bytes());
        hasher.update(encrypted_hash.as_bytes());
        hasher.update(&request_info_hash);
        hasher.finalize_xof().fill(&mut okm);
        k256::Scalar::from_okm(&okm.into())
    };

    let s = k - secret.0 * e;
    let mut ret = [0; 64];
    ret[0..32].copy_from_slice(&e.to_bytes());
    ret[32..].copy_from_slice(&s.to_bytes());
    ret
}
