mod cid;
mod indexer;
mod resolver;

use std::net::IpAddr;

pub use cid::{CIDResolver, Cid};
pub use resolver::{Cluster, Resolver};

const FLEEK_NETWORK_FILTER: &[u8] = b"FleekNetwork";

pub(crate) type Key = IpAddr;
