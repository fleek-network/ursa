use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct IndexerResponse {
    #[serde(rename = "MultihashResults")]
    pub multihash_results: Vec<MultihashResult>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct MultihashResult {
    #[serde(rename = "Multihash")]
    multihash: String,
    #[serde(rename = "ProviderResults")]
    pub provider_results: Vec<ProviderResult>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ProviderResult {
    #[serde(rename = "ContextID")]
    context_id: String,
    #[serde(rename = "Metadata")]
    pub metadata: String,
    #[serde(rename = "Provider")]
    pub provider: AddrInfo,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AddrInfo {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "Addrs")]
    pub addrs: Vec<Multiaddr>,
}
