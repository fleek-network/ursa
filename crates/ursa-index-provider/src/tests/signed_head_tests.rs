// https://github.com/MarcoPolo/http-index-provider-example/blob/main/src/signed_head.rs

#[cfg(test)]
mod tests {
    use libipld::Cid;
    use libp2p::identity::Keypair;

    use crate::signed_head::SignedHead;

    #[test]
    fn test_decode_signed_msg() {
        // From Go implementation
        let signed_msg = r#"{"head":{"/":"bafybeicyhbhhklw3kdwgrxmf67mhkgjbsjauphsvrzywav63kn7bkpmqfa"},"pubkey":{"/":{"bytes":"CAESIJSklColz5Jq+bVsKPQpxmEwo9avM7y/vVkYSDttBWLI"}},"sig":{"/":{"bytes":"1S4p2vHPXobyPnspQWkCHMjf2n5qQCMb+OehDjUnQbRil3qf95g87VNcIxl6hr66zmhBeJ7h+Y6UnUUhnUMZAQ"}}}"#;

        let signed_head: SignedHead = serde_json::from_str(signed_msg).expect("deser failed");
        let signed_head_encoded = serde_json::to_string(&signed_head).expect("ser failed");
        // Round trip to test
        let signed_head: SignedHead =
            serde_json::from_str(&signed_head_encoded).expect("round trip failed");

        let (pk, head) = signed_head.open().expect("failed to open signed_head");

        println!("{head:?}\n{pk:?}");
    }

    #[test]
    fn test_sign_head() {
        let kp = Keypair::generate_ed25519();
        let cid = Cid::try_from("bafybeicyhbhhklw3kdwgrxmf67mhkgjbsjauphsvrzywav63kn7bkpmqfa")
            .expect("failed to parse cid");
        let signed_head = SignedHead::new(&kp, cid).expect("failed to sign head");
        let signed_head_encoded = serde_json::to_string(&signed_head).expect("ser failed");
        let signed_head: SignedHead =
            serde_json::from_str(&signed_head_encoded).expect("deser failed");

        let (pk, head) = signed_head.open().expect("failed to open signed_head");

        assert_eq!(head, cid);
        assert_eq!(pk, kp.public());
    }
}
