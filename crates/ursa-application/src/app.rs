use crate::env::AtomoEnv;
use crate::interface::application::{App, ApplicationQuery, ApplicationUpdate};

pub struct Application {
    inner: App,
}

impl Application {
    /// Creates and runs the application
    pub fn new() -> Self {
        let env = AtomoEnv::new();

        Self {
            inner: App::new(env),
        }
    }

    /// Get the query port. There can be multiple clones of this port and it can be called multiple times
    pub fn get_query_port(&self) -> ApplicationQuery {
        self.inner.get_query_port()
    }

    /// Get the update port. There should only be one update port and it should be handed to consensus
    pub fn get_update_port(&self) -> ApplicationUpdate {
        self.inner.get_query_port()
    }

    /// Helper function to recieve the update and query port
    pub fn get_ports(&self) -> (ApplicationUpdate, ApplicationQuery) {
        (self.inner.get_update_port(), self.inner.get_query_port())
    }
}
