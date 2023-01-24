// https://github.com/MarcoPolo/http-index-provider-example/blob/main/src/signed_head.rs

use base64;
use libipld::{cid, Cid};
use libp2p::{
    core::{identity::Keypair, PublicKey},
    identity::error::SigningError,
};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use serde_with::serde_as;
use thiserror::Error;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct SignedHead {
    #[serde_as(as = "CidAsMap")]
    head: Cid,
    #[serde_as(as = "BytesAsMap")]
    sig: Vec<u8>,
    #[serde_as(as = "BytesAsMap")]
    pubkey: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum SignedHeadError {
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Invalid public key")]
    InvalidPublicKey,
}

impl SignedHead {
    pub fn new(signing_key: &Keypair, cid: Cid) -> Result<Self, SigningError> {
        let sig = signing_key.sign(&cid.to_bytes())?;
        Ok(SignedHead {
            head: cid,
            pubkey: signing_key.public().to_protobuf_encoding(),
            sig,
        })
    }

    pub fn open(self) -> Result<(PublicKey, Cid), SignedHeadError> {
        let pk = PublicKey::from_protobuf_encoding(&self.pubkey)
            .map_err(|_| SignedHeadError::InvalidPublicKey)?;
        let valid = pk.verify(&self.head.to_bytes(), &self.sig);
        if !valid {
            return Err(SignedHeadError::InvalidSignature);
        }

        Ok((pk, self.head))
    }
}

serde_with::serde_conv!(BytesAsMap, Vec<u8>, from_bytes_to_map, from_map_to_bytes);
fn from_bytes_to_map(bytes: &Vec<u8>) -> Map<String, serde_json::Value> {
    let mut m = Map::new();
    let mut bytes_map = Map::new();
    bytes_map.insert("bytes".into(), base64::encode(bytes).into());
    m.insert("/".into(), bytes_map.into());
    m
}

fn from_map_to_bytes(
    value: Map<String, serde_json::Value>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let bytes_str = value
        .get("/")
        .ok_or("Missing link key")?
        .get("bytes")
        .ok_or("Missing bytes key")?
        .as_str()
        .ok_or("missing bytes string")?;

    Ok(base64::decode(bytes_str).map_err(|e| format!("{e}"))?)
}

serde_with::serde_conv!(CidAsMap, Cid, from_cid_to_map, from_map_to_cid);
fn from_cid_to_map(cid: &Cid) -> Map<String, serde_json::Value> {
    let mut map = Map::new();
    map.insert("/".to_string(), cid.to_string().into());
    map
}

fn from_map_to_cid(value: Map<String, serde_json::Value>) -> Result<Cid, cid::Error> {
    let cid_str = value
        .get("/")
        .ok_or_else(|| {
            cid::Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Cid entry is missing",
            ))
        })?
        .as_str()
        .ok_or_else(|| {
            cid::Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Cid str is missing",
            ))
        })?;

    let cid = Cid::try_from(cid_str)?;

    Ok(cid)
}
#[cfg(test)]
#[path = "tests/signed_head_tests.rs"]
mod signed_head_tests;
