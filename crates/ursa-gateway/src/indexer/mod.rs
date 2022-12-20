pub mod model;

use crate::indexer::model::IndexerResponse;
use libp2p::multiaddr::Protocol;
use rand::Rng;
use std::net::{IpAddr, SocketAddr};
use tracing::error;

// Randomly chooses a provider and returns its addresses.
pub fn get_provider(indexer_response: &IndexerResponse) -> Option<Vec<SocketAddr>> {
    let provider = &indexer_response
        .multihash_results
        .get(0)
        .unwrap()
        .provider_results;

    if !provider.is_empty() {
        let len = provider.len();
        let mut rng = rand::thread_rng();
        let rand_i = rng.gen_range(std::ops::Range { start: 0, end: len });
        let multiaddrs = provider[rand_i as usize].provider.addrs.iter();

        let mut provider_addrs = Vec::new();
        for maddr in multiaddrs {
            let components = maddr.iter().collect::<Vec<_>>();
            let ip = match components.get(0) {
                Some(Protocol::Ip4(ip)) => ip,
                _ => {
                    error!("failed to get ip");
                    return None;
                }
            };
            let port = match components.get(1) {
                Some(Protocol::Tcp(port)) => port,
                _ => {
                    error!("failed to get port");
                    return None;
                }
            };
            provider_addrs.push(SocketAddr::new(IpAddr::from(*ip), *port));
        }

        Some(provider_addrs)
    } else {
        None
    }
}
