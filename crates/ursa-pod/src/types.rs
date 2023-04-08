use std::fmt::Display;

pub type EpochNonce = u64;
pub type Secp256k1AffinePoint = [u8; 33];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blake3Cid(pub [u8; 32]);

impl Display for Blake3Cid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:x}")?;
        }
        Ok(())
    }
}

pub type Secp256k1PublicKey = Secp256k1AffinePoint;
pub type SchnorrSignature = [u8; 64];

pub type BlsPublicKey = [u8; 48];
pub type BlsSignature = [u8; 96];
