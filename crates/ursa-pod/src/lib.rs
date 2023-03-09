use elliptic_curve::group::GroupEncoding;
use elliptic_curve::hash2curve::{hash_to_field, ExpandMsgXmd};
use elliptic_curve::Field;
use rand_core::{block::BlockRngCore, OsRng, SeedableRng};
use sha2::Sha256;
use std::iter::zip;

pub struct Request {}
pub struct SecretKey(pub k256::Scalar);

/// Encrypt a block of data and write the result to the provided mutable slice, this function will
/// return an additional 64 byte that should be send to
// TODO(qti3e) If possible in future make this function encrypt in place.
pub fn encrypt_block(
    secret: &SecretKey,
    req: &Request,
    buffer: &[u8],
    result: &mut [u8],
) -> [u8; 64] {
    // DO NOT CHANGE THIS UNLESS YOU MAKE SURE THE TRANSMUTE LOGIC IS ALSO WORKING.
    const BLOCK_SIZE: usize = 64;

    assert_eq!(
        buffer.len(),
        result.len(),
        "plaintext and result buffer must be the same size."
    );

    let mut buffer_iter = buffer.chunks_exact(BLOCK_SIZE);
    let mut result_iter = result.chunks_exact_mut(BLOCK_SIZE);

    let mut hc_block = [0u8; BLOCK_SIZE];
    let mut hc_rng = rand_hc::Hc128Core::from_seed([0; 32]);

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
        // TODO(qti3e) include message and request hash.
        let mut c = [k256::Scalar::ZERO];
        hash_to_field::<ExpandMsgXmd<Sha256>, k256::Scalar>(
            &[&r_compressed, hash.as_bytes()],
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

#[test]
fn x() {
    use benchmarks_utils::*;
    const SIZE: usize = 256 * KB;

    let mut data = [0u8; SIZE];
    let mut result = [0u8; SIZE];

    OsRng.fill_bytes(&mut data);

    let s_key = SecretKey(k256::Scalar::random(OsRng));

    let mut result = mk_vec(SIZE);
    encrypt_block(&s_key, &Request {}, &data, &mut result);
}
