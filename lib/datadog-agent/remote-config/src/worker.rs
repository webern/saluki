use async_trait::async_trait;
use saluki_common::sync::shutdown::ShutdownHandle;
use saluki_core::runtime::{InitializationError, Supervisable, SupervisorFuture};
use saluki_error::GenericError;

/// Drives polling and delivery for a [`RemoteConfigurationClient`](crate::RemoteConfigurationClient).
///
/// A supervisor can own and restart the worker, or the caller can drive it directly with [`run`](Self::run).
///
/// The worker does not exit on connection failures, on the Agent having remote configuration disabled, on the Agent
/// reporting its configuration expired, or on a panic in a subscriber's decoder. It keeps polling through all of them,
/// so it exits only on a failure in the client itself.
///
/// A restart keeps every subscription and each product's last accepted snapshot, and discards the protocol state, so
/// the restarted worker fetches and decodes everything again. Subscribers may therefore see a snapshot equal to the one
/// they already hold.
// TODO: hold the subscription registry and client ID behind an `Arc` shared with the client, so they survive restarts.
// TODO: poll on the schedule documented on `RcClientConfiguration`, using `saluki_io`'s `ExponentialBackoff`, and poll
// immediately when a subscribe wakes the worker.
// TODO: treat `Unimplemented` as a lasting poll failure: wait `max_backoff`, and log once when entered and once on
// recovery.
// TODO: treat `CONFIG_STATUS_EXPIRED` as an assignment of nothing: advance the cursor, forget cached files, and decode
// every affected product as empty.
// TODO: decode a product only when its paths or hashes change, and catch panics from `decode` and `build` with
// `catch_unwind(AssertUnwindSafe(..))`, rejecting the whole snapshot without publishing.
// TODO: drop products whose last subscription clone was dropped from the request and from reporting.
// TODO: emit metrics for poll outcomes, rejections by product and stage, snapshots published, and time since the last
// successful poll.
#[non_exhaustive]
pub struct RemoteConfigurationWorker {}

impl RemoteConfigurationWorker {
    /// Runs the worker without a supervisor.
    ///
    /// # Errors
    ///
    /// Returns an error only on a failure in the client itself. Connection failures, remote configuration being
    /// disabled on the Agent, and rejected or panicking decoders are handled without returning.
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
        // TODO: retain shutdown in the returned future and drive the same polling logic as `run`, starting from fresh
        // protocol state on every call.
        todo!()
    }
}
