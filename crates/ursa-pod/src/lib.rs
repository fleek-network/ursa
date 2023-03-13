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

    // To leverage SIMD we want the compiler to be able to auto vectorize some of our inner
    // for-loops, to achieve this the outer loop works on *chunks* of data, each chunk is fixed
    // size, this setup allows the compiler to know that the inner for loop always has a constant
    // number of iterations before termination and can unroll the loop statically.
    //
    // The number 64 is chosen as the step size, because we're working on u8 slices, meaning each
    // inner loop will work on 512bit and that should allow the compiler to use avx512 or similar
    // instructions and produce the most efficient code. Benchmarks have proven this point.
    //
    // On the other side, the Hc128 implementation which we're using is using blocks of [u32; 16]
    // which is 64 bytes and this allows us to have a single call to its generate function in the
    // outer loop once per iteration.
    //
    // DO NOT CHANGE THIS UNLESS YOU MAKE SURE THE TRANSMUTE LOGIC IS ALSO WORKING.
    const BLOCK_SIZE: usize = 64;

    // Use the same blake3 reader for computing the hash and the mapping to the curve.
    // this reader is stateful but has a cheap clone.
    let request_info_reader = req.hash_xof();

    // blake3 hash of the request information, because of how the xof works the first 32 bytes
    // we read from the blake3's OutputReader is exactly the blake3 hash of the content.
    //
    // This clause clones the `request_info_reader` because we want to preserve the fresh xof
    // for the next block where we compute a random point on the curve from the request info.
    let request_info_hash = {
        let mut buffer = [0; 32];
        request_info_reader.clone().fill(&mut buffer);
        buffer
    };

    // Map the request information to a point on the curve, the generated point has an unknown
    // discrete log.
    //
    // We do this by expanding the blake3 hash to 96 bytes using the xof reader, the first 48
    // bytes is hashed to one field element and the other 48 byte is also hashed to another
    // point, the result is the addition of these two points.
    let request_info_on_curve = {
        let mut buffer = [0; 48];
        let mut reader = request_info_reader;

        reader.fill(&mut buffer);
        let q0 = k256::FieldElement::from_okm(&buffer.into()).map_to_curve();

        reader.fill(&mut buffer);
        let q1 = k256::FieldElement::from_okm(&buffer.into()).map_to_curve();

        q0 + q1
    };

    // We have: $ k=H(secret * Req) $. Here we multiply the request as a point on the curve with
    // the secret share and hash the compressed representation of the result via blake3. This 32
    // byte array is then used as the seed to the Hc128 rng and can give us a simple encryption.
    let seed = {
        let encryption_key = request_info_on_curve * secret.0;
        let encoded_point = encryption_key.to_affine().to_encoded_point(true);
        blake3::hash(encoded_point.as_bytes())
    };

    let mut hc_block = [0u8; BLOCK_SIZE];
    let mut hc_rng = rand_hc::Hc128Core::from_seed(*seed.as_bytes());

    // Create two chunked iterators over the plaintext and ciphertext.
    let mut buffer_iter = buffer.chunks_exact(BLOCK_SIZE);
    let mut result_iter = result.chunks_exact_mut(BLOCK_SIZE);

    // Encryption loop: iterate over the data in chunks of 64 byte and xor the buffer with the
    // HC128's current block.
    for (buffer_block, result_block) in zip(&mut buffer_iter, &mut result_iter) {
        // SAFETY: sizeof(u32; 16) == sizeof(u8; 64).
        unsafe {
            debug_assert!(BLOCK_SIZE == 32 / 8 * 16);
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

    // Perform the encryption for the remaining bytes if there are any.
    if buffer.len() % BLOCK_SIZE > 0 {
        // SAFETY: sizeof(u32; 16) == sizeof(u8; 64).
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

    // Compute the hash of the encrypted data. The node needs to sign-off the response, so here
    // we hash what we're gonna deliver to the user.
    let ciphertext_hash = {
        let mut hasher = blake3::Hasher::new();
        hasher.update_rayon(&result);
        hasher.finalize()
    };

    // # Schnorr commitment to the encrypted data.
    // To commit to a certain encryption, we simply produce a Schnorr signature with out private
    // key, the message is $ m = REQUEST_INFO_HASH . CIPHERTEXT_HASH $.

    let k = k256::Scalar::random(OsRng);
    let r = (k256::AffinePoint::GENERATOR * k).to_affine();

    // Compute the challenge. It is basically the hash of everything that's publicly available.
    let e = {
        let mut okm: [u8; 48] = [0; 48];
        let mut hasher = blake3::Hasher::new_derive_key(SCHNORR_CHALLENGE_DOMAIN_SEP);
        hasher.update(r.to_encoded_point(true).as_bytes());
        hasher.update(ciphertext_hash.as_bytes());
        hasher.update(&request_info_hash);
        hasher.finalize_xof().fill(&mut okm);
        k256::Scalar::from_okm(&okm.into())
    };

    let s = k - secret.0 * e;

    // The response is a 64-byte tag, the two 32-byte chunks are the (e, s) of the schnorr
    // signature in big endian representation.
    let mut ret = [0; 64];
    ret[0..32].copy_from_slice(&e.to_bytes());
    ret[32..].copy_from_slice(&s.to_bytes());
    ret
}
