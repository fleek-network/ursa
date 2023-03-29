pub type EpochNonce = u64;
pub type Secp256k1AffinePoint = [u8; 33];

pub type Blake3CID = [u8; 32];

pub type Secp256k1PublicKey = Secp256k1AffinePoint;
pub type SchnorrSignature = [u8; 64];

pub type BLSPublicKey = [u8; 48];
pub type BLSSignature = [u8; 96];
