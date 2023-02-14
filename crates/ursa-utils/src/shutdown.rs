//! The shutdown module makes it easy for us to have one source for sending shutdowns
//! signal to every sub service and have proper and graceful shutdowns in all of Ursa
//! services.

use std::sync::Arc;
use tokio::signal::ctrl_c;
use tokio::sync::Notify;

/// The shutdown controller can be used to be passed around in a program and provide
/// a unified signal for shutting down every single service and provide the application
/// with graceful shutdowns.
#[derive(Clone, Default)]
pub struct ShutdownController {
    notify: Arc<Notify>,
}

impl ShutdownController {
    /// Install the handler for control c to submit trigger this shutdown handler.
    pub fn install_ctrl_c_handler(&self) {
        let notify = self.notify.clone();
        tokio::task::spawn(async move {
            ctrl_c().await.expect("Failed to setup control-c handler.");
            notify.notify_waiters();
        });
    }

    /// Manually send the shutdown signal.
    pub fn shutdown(&self) {
        // just forward the call to the notify_waiters which wakes up all of
        // the waiters.
        self.notify.notify_waiters();
    }

    /// Wait for the shutdown signal to be sent.
    pub async fn wait_for_shutdown(self) {
        self.notify.notified().await;
    }
}
