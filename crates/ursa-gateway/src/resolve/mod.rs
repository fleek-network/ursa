mod cid;
mod indexer;
mod resolver;

use std::net::IpAddr;

const FLEEK_NETWORK_FILTER: &[u8] = b"FleekNetwork";

pub(crate) type Key = IpAddr;
