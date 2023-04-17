//! The domain separators defined for different states of the protocol to be used with
//! Blake3 `keyed_hash`.

use hex_literal::hex;

/// Should be used to compress a request info raw bytes.
pub const HASH_REQUEST_INFO: [u8; 32] =
    hex!("4D85E693C2204AE36F69DE8664498AEFF5CA26DD350D9D01C81D818F589C3C8E");

/// Used for generating the hashes to drive a symmetric key.
pub const HASH_TO_SYMMETRIC_KEY: [u8; 32] =
    hex!("F9C8329F93E84FFE57AB9963D86B1F8369665FB741381671AF8B335C9F0907DA");

/// Domain separator for hashing the ciphertext.
pub const CIPHERTEXT_DIGEST: [u8; 32] =
    hex!("4D4B3F8801E1C8A92DD137E5A546EC8C6147357ADA43B399FB681E929C57ED9B");

/// Domain separator used in the `sign_ciphertext` procedure.
pub const CIPHERTEXT_COMMITMENT: [u8; 32] =
    hex!("9EA73937117EE63FDFE7D69C8A02A189062A2686F36D4BDFD6DFAE2FA8A50442");

#[cfg(test)]
mod tests {
    use super::*;
    use blake3::derive_key;

    #[test]
    fn hash_request_info_key() {
        let key = derive_key("HASH_REQUEST_INFO", b"FLEEK-NETWORK-UFDP");
        assert_eq!(
            key,
            HASH_REQUEST_INFO,
            "expected='{}'",
            blake3::Hash::from(key).to_hex()
        );
    }

    #[test]
    fn hash_to_symmetric_key_key() {
        let key = derive_key("HASH_TO_SYMMETRIC_KEY", b"FLEEK-NETWORK-UFDP");
        assert_eq!(
            key,
            HASH_TO_SYMMETRIC_KEY,
            "expected='{}'",
            blake3::Hash::from(key).to_hex()
        );
    }

    #[test]
    fn ciphertext_hash_key() {
        let key = derive_key("CIPHERTEXT_DIGEST", b"FLEEK-NETWORK-UFDP");
        assert_eq!(
            key,
            CIPHERTEXT_DIGEST,
            "expected='{}'",
            blake3::Hash::from(key).to_hex()
        );
    }

    #[test]
    fn ciphertext_commitment_key() {
        let key = derive_key("CIPHERTEXT_COMMITMENT", b"FLEEK-NETWORK-UFDP");
        assert_eq!(
            key,
            CIPHERTEXT_COMMITMENT,
            "expected='{}'",
            blake3::Hash::from(key).to_hex()
        );
    }
}
