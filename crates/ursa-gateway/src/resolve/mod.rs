mod cid;
mod indexer;
mod resolver;

use std::net::IpAddr;
use thiserror::Error;

pub use cid::{CIDResolver, Cid};
pub use resolver::{Cluster, Resolver};

pub(crate) type Key = IpAddr;

#[derive(Debug, Error)]
pub enum ResolutionError {
    #[error("invalid response from indexer")]
    InvalidResponseFromIndexer,
    #[error("failed to resolve: {0}")]
    Internal(String),
}
