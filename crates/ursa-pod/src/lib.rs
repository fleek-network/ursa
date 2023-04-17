//! Implementation of Ursa's fair delivery protocol.

/// UFDP codec implementation
pub mod codec;

/// UFDP client implementation
#[cfg(feature = "client")]
pub mod client;

/// UFDP server implementation
#[cfg(feature = "server")]
pub mod server;

/// Implementation of the cryptographic primitives and routines
/// for UFDP.
pub mod crypto;

/// The primitives for the verifiable streaming on top of Blake3.
pub mod tree;

/// UFDP types
pub mod types;

/// Reexport of the Blake3 we use.
pub use blake3;
