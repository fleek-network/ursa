pub mod model;

use crate::indexer::model::IndexerResponse;
use libp2p::multiaddr::Protocol;
use rand::Rng;
use std::net::SocketAddrV4;
use tracing::error;

// Randomly chooses a provider and returns its addresses.
pub fn get_provider(indexer_response: IndexerResponse) -> Option<Vec<SocketAddrV4>> {
    // We expect that MultihashResults will have at most 1 element.
    let providers = match indexer_response.multihash_results.get(0) {
        Some(result) => &result.provider_results,
        None => {
            error!("indexer result did not contain a multi-hash result");
            return None;
        }
    };

    if !providers.is_empty() {
        let len = providers.len();
        let mut rng = rand::thread_rng();
        let rand_i = rng.gen_range(std::ops::Range { start: 0, end: len });
        let multiaddrs = providers[rand_i as usize].provider.addrs.iter();

        let mut provider_addrs = Vec::new();
        for maddr in multiaddrs {
            let components = maddr.iter().collect::<Vec<_>>();
            let ip = match components.get(0) {
                Some(Protocol::Ip4(ip)) => ip,
                _ => {
                    error!("skipping address {maddr}");
                    continue;
                }
            };

            let port = match components.get(1) {
                Some(Protocol::Tcp(port)) => port,
                _ => {
                    error!("skipping address {maddr} without port");
                    continue;
                }
            };

            provider_addrs.push(SocketAddrV4::new(*ip, *port));
        }
        if provider_addrs.is_empty() {
            error!("failed to get a valid address for provider.");
            return None;
        }
        Some(provider_addrs)
    } else {
        error!("multi-hash result did not contain a provider");
        None
    }
}
