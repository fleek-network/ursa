pub mod model;

use crate::indexer::model::IndexerResponse;
use anyhow::{anyhow, bail, Error};
use libp2p::multiaddr::Protocol;
use std::net::SocketAddrV4;
use tracing::warn;

// Chooses a provider and returns its addresses.
pub fn choose_provider(indexer_response: IndexerResponse) -> Result<Vec<SocketAddrV4>, Error> {
    // We expect that MultihashResults will have at most 1 element.
    let providers = &indexer_response
        .multihash_results
        // Just choose the first one for now.
        .get(0)
        .ok_or_else(|| anyhow!("indexer result did not contain a multi-hash result"))?
        .provider_results;

    if providers.is_empty() {
        bail!("multi-hash result did not contain a provider")
    }

    let multiaddrs = providers[0].provider.addrs.iter();

    let mut provider_addrs = Vec::new();
    for maddr in multiaddrs {
        let mut components = maddr.iter();
        let ip = match components.next() {
            Some(Protocol::Ip4(ip)) => ip,
            _ => {
                warn!("skipping address {maddr}");
                continue;
            }
        };

        let port = match components.next() {
            Some(Protocol::Tcp(port)) => port,
            _ => {
                warn!("skipping address {maddr} without port");
                continue;
            }
        };

        provider_addrs.push(SocketAddrV4::new(ip, port));
    }

    if provider_addrs.is_empty() {
        bail!("failed to get a valid address for provider.");
    }

    Ok(provider_addrs)
}
