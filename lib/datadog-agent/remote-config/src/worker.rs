use async_trait::async_trait;
use saluki_common::sync::shutdown::ShutdownHandle;
use saluki_core::runtime::{InitializationError, Supervisable, SupervisorFuture};
use saluki_error::GenericError;

/// Drives polling and delivery for a [`RemoteConfigurationClient`](crate::RemoteConfigurationClient).
///
/// A supervisor can own and restart the worker, or the caller can drive it directly with [`run`](Self::run).
#[non_exhaustive]
pub struct RemoteConfigurationWorker {}

impl RemoteConfigurationWorker {
    /// Runs the worker without a supervisor.
    ///
    /// # Errors
    ///
    /// Returns an error if the worker cannot recover and continue polling.
    pub async fn run(self) -> Result<(), GenericError> {
        todo!()
    }
}

#[async_trait]
impl Supervisable for RemoteConfigurationWorker {
    fn name(&self) -> &str {
        "remote-config"
    }

    async fn initialize(&self, _process_shutdown: ShutdownHandle) -> Result<SupervisorFuture, InitializationError> {
        // TODO: retain shutdown in the returned future and drive the same polling logic as `run`.
        todo!()
    }
}
