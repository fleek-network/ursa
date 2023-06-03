mod cid;
mod indexer;
mod resolver;

use hyper::{Body, Request};
use std::net::IpAddr;
use thiserror::Error;
use tower::balance::p2c::MakeBalance;

pub use cid::{CIDResolver, Cid};
pub use resolver::{Cluster, Config, Resolve};

pub(crate) type Key = IpAddr;

/// Resolves to a set of backend services.
pub type MakeBackend<R> = MakeBalance<Resolve<R>, Request<Body>>;

#[derive(Debug, Error)]
pub enum ResolutionError {
    #[error("invalid response from indexer")]
    InvalidResponseFromIndexer,
    #[error("failed to resolve: {0}")]
    Internal(String),
}
