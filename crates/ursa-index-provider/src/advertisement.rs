use libipld::multihash::{Code, MultihashDigest};
use libipld_core::ipld::Ipld;
use libp2p::{
    core::{signed_envelope, SignedEnvelope},
    identity::Keypair,
    PeerId,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A chunk can hold maximum 400 MB in entries. An entry being 64 bytes
/// max number of entries 6,250,000
pub const MAX_ENTRIES: usize = 6250000;
const AD_SIGNATURE_CODEC: &str = "/indexer/ingest/adSignature";
const AD_SIGNATURE_DOMAIN: &str = "indexer";

#[allow(non_snake_case)]
#[derive(Serialize)]
struct Metadata {
    // ProtocolID defines the protocol used for data retrieval.
    ProtocolID: u128,
    // Size of the content.
    Size: u64,
    // Data is specific to the identified protocol, and provides data, or a
    // link to data, necessary for retrieval.
    Data: Vec<u8>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Advertisement {
    /// PreviousID is an optional link to the previous advertisement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub PreviousID: Option<Ipld>,
    /// Provider is the peer ID of the host that provides this advertisement.
    pub Provider: String,
    /// Addresses is the list of multiaddrs as strings from which the advertised content is retrievable.
    pub Addresses: Vec<String>,
    /// Advertisement signature.
    pub Signature: Ipld,
    /// Entries with a link to the list of CIDs
    /// Entries is a link to a data structure that contains the advertised multihashes.
    pub Entries: Option<Ipld>,
    /// ContextID is the unique identifier for the collection of advertised multihashes.
    pub ContextID: Ipld,
    /// Metadata captures contextual information about how to retrieve the advertised content
    pub Metadata: Ipld,
    /// IsRm specifies whether this advertisement represents the content are no longer retrievable fom the provider.
    pub IsRm: bool,
}
impl Advertisement {
    pub fn new(
        context_id: Vec<u8>,
        provider: PeerId,
        addresses: Vec<String>,
        is_rm: bool,
        content_size: u64,
    ) -> Self {
        // prtocolid for bitswap
        let raw_metadata = Metadata {
            ProtocolID: 0x0900,
            Data: b"FleekNetwork".to_vec(),
            Size: content_size,
        };
        let metadata = bincode::serialize(&raw_metadata).unwrap();

        Self {
            PreviousID: None,
            Provider: provider.to_base58(),
            Addresses: addresses,
            Signature: Ipld::Bytes(vec![]),
            Entries: None,
            Metadata: Ipld::Bytes(metadata),
            ContextID: Ipld::Bytes(context_id),
            IsRm: is_rm,
        }
    }
    pub fn sign(&self, signing_key: &Keypair) -> Result<SignedEnvelope, AdSigError> {
        SignedEnvelope::new(
            signing_key,
            AD_SIGNATURE_DOMAIN.into(),
            AD_SIGNATURE_CODEC.into(),
            self.sig_payload()?,
        )
        .map_err(AdSigError::SigningError)
    }

    /// computes a signature over all of these fields
    /// https://github.com/MarcoPolo/http-index-provider-example/blob/6ebda4211c93324405c827b5ffc46c513741efa8/src/advertisement.rs#L49
    fn sig_payload(&self) -> Result<Vec<u8>, AdSigError> {
        let mut previous_id_bytes = match &self.PreviousID {
            Some(Ipld::Link(link)) => Ok(link.to_bytes()),
            None => Ok(vec![]),
            _ => Err(AdSigError::InvalidPreviousID),
        }?;

        let mut entrychunk_link_bytes: Vec<u8> = match &self.Entries {
            Some(Ipld::Link(link)) => Ok(link.to_bytes()),
            None => Ok(vec![]),
            _ => Err(AdSigError::InvalidEntryChunkLink),
        }?;

        let metadata = match &self.Metadata {
            Ipld::Bytes(b) => Ok(b),
            _ => Err(AdSigError::InvalidMetadata),
        }?;

        let is_rm_payload = if self.IsRm { [1] } else { [0] };

        let mut payload: Vec<u8> = Vec::with_capacity(
            previous_id_bytes.len()
                + entrychunk_link_bytes.len()
                + self.Provider.len()
                + self.Addresses.iter().map(|s| s.len()).sum::<usize>()
                + metadata.len()
                + is_rm_payload.len(),
        );

        payload.append(&mut previous_id_bytes);
        payload.append(&mut entrychunk_link_bytes);
        payload.extend_from_slice(self.Provider.as_bytes());
        self.Addresses
            .iter()
            .for_each(|s| payload.extend_from_slice(s.as_bytes()));
        payload.extend_from_slice(metadata);
        payload.extend_from_slice(&is_rm_payload);

        Ok(Code::Sha2_256.digest(&payload).to_bytes())
    }
}

#[derive(Debug, Error)]
pub enum AdSigError {
    #[error("Invalid Previous ID")]
    InvalidPreviousID,
    #[error("Invalid Entry Chunk Link")]
    InvalidEntryChunkLink,
    #[error("Invalid Metadata")]
    InvalidMetadata,
    #[error("Missing Signature")]
    MissingSig,
    #[error("Failed to sign advertisement: {0}")]
    SigningError(libp2p::identity::error::SigningError),
    #[error("Failed to decode sig: {0}")]
    DecodingError(signed_envelope::DecodingError),
    #[error("Failed to read signed payload: {0}")]
    ReadPayloadError(signed_envelope::ReadPayloadError),
    #[error("Payload did not match expected")]
    PayloadDidNotMatch,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug)]
/// EntryChunk captures a chunk in a chain of entries that collectively contain the multihashes
/// advertised by an Advertisement.
pub struct EntryChunk {
    /// Entries represent the list of multihashes in this chunk.
    Entries: Vec<Ipld>,
    /// Next is an optional link to the next entry chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    Next: Option<Ipld>,
}

impl EntryChunk {
    pub fn new(entries: Vec<Ipld>, next: Option<Ipld>) -> Self {
        Self {
            Entries: entries,
            Next: next,
        }
    }
}
