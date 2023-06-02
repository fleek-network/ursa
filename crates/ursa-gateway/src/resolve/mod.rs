mod cid;
mod indexer;
mod resolver;

use std::net::IpAddr;

pub use cid::{CIDResolver, Cid};
pub use resolver::{Cluster, Resolver};

pub(crate) type Key = IpAddr;
