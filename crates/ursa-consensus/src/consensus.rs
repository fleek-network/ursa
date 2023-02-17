use crate::{config::ConsensusConfig, narwhal::NarwhalService};

// what do we need for this file to work and be complete?
// - A mechanism to dynamically move the epoch forward and changing the committee dynamically.
//    Each epoch has a fixed committee. The committee only changes per epoch.
// - Manage different stores for each epoch.
// - Restore the latest committee and epoch information from a persistent database.
// - Restart the narwhal service for each new epoch.
// - Execution engine with mpsc or a normal channel to deliver the transactions to abic.
//
// TBD:
// - Do we need a catch up process here in this file?
// - Where will we be doing the communication with execution engine from this file?
//
// But where should the config come from?

/// The consensus layer, which wraps a narwhal service and moves the epoch forward.
pub struct Consensus {
    narwhal: NarwhalService,
}

impl Consensus {
    pub fn new(config: ConsensusConfig) {}

    pub async fn start(&self) {}

    pub async fn shutdown(&self) {}
}
