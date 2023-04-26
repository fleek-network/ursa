use std::fmt::Display;

pub type EpochNonce = u64;
pub type Secp256k1AffinePoint = [u8; 33];

pub type Secp256k1PublicKey = Secp256k1AffinePoint;
pub type SchnorrSignature = [u8; 64];

pub type BlsPublicKey = [u8; 48];
pub type BlsSignature = [u8; 96];
