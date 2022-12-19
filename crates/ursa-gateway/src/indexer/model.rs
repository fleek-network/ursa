use serde::{Deserialize, Serialize};
use libp2p::Multiaddr;

#[derive(Deserialize, Serialize, Debug)]
pub struct IndexerResponse {
    #[serde(rename = "MultihashResults")]
    multihash_results: Vec<MultihashResult>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct MultihashResult {
    #[serde(rename = "Multihash")]
    multihash: String,
    #[serde(rename = "ProviderResults")]
    provider_results: Vec<ProviderResult>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ProviderResult {
    #[serde(rename = "ContextID")]
    context_id: String,
    #[serde(rename = "Metadata")]
    metadata: String,
    #[serde(rename = "Provider")]
    provider: AddrInfo,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AddrInfo {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "Addrs")]
    addrs: Vec<Multiaddr>,
}
