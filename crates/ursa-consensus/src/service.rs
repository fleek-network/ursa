use crate::config::ConsensusConfig;
use narwhal_config::Epoch;

/// Manages running the narwhal and bullshark as a service.
pub struct ConsensusService {}

///
pub struct ServiceConfig {
    epoch: Epoch,
}

impl ConsensusService {
    pub async fn start(&self) {}
}
