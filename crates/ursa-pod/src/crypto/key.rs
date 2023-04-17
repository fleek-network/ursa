use zeroize::Zeroize;

/// The trait for a secret key with `SIZE` many bytes.
pub trait SecretKey<const SIZE: usize>: FixedSizeEncoding<SIZE> + Zeroize {}

/// The trait for a public key with `SIZE` many bytes.
pub trait PublicKey<const SIZE: usize>: FixedSizeEncoding<SIZE> + Zeroize {}

pub trait FixedSizeEncoding<const SIZE: usize>: Sized {
    /// Deserialize the data from an array of the given size, returns
    /// `None` if the data is not valid.
    fn try_from_bytes(bytes: &[u8; SIZE]) -> Option<Self>;

    /// Serialize the data to an array of the given size.
    fn to_bytes(&self) -> [u8; SIZE];
}
