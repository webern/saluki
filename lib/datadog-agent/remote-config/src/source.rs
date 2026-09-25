//! The worker's one dependency on the Agent, separated so that tests can replace the Agent.

use async_trait::async_trait;
use datadog_agent_commons::ipc::client::RemoteAgentClient;
use datadog_protos::remote_config::{ClientGetConfigsRequest, ClientGetConfigsResponse};
use saluki_error::GenericError;

/// Answers the worker's configuration polls.
///
/// The production implementation forwards to the Agent's `ClientGetConfigs` call. The crate's own tests implement it
/// with a scripted fake that returns prepared responses and errors in order and records every request, so the worker's
/// polling, integrity checks, status reporting, and backoff can be tested without an Agent, and under a paused clock.
///
/// This trait is private. Subscribers never see it: [`RemoteConfigurationClient::new`] takes a [`RemoteAgentClient`],
/// and the worker holds the source as a `Box<dyn ConfigSource>` so that no public type gains a type parameter.
///
/// [`RemoteConfigurationClient::new`]: crate::RemoteConfigurationClient::new
// TODO: name not locked.
// TODO: remove dead_code guard once the worker polls through this trait.
#[allow(dead_code)]
#[async_trait]
pub(crate) trait ConfigSource: Send + 'static {
    /// Sends one poll and returns the Agent's response.
    async fn get_configs(&mut self, request: ClientGetConfigsRequest) -> Result<ClientGetConfigsResponse, FetchError>;
}

/// Why a poll produced no response.
///
/// The worker's reaction depends only on which of these occurred, so the gRPC status codes stay inside the production
/// [`ConfigSource`].
// TODO: remove dead_code guard once the worker polls through `ConfigSource`.
#[allow(dead_code)]
pub(crate) enum FetchError {
    /// The Agent has Remote Configuration disabled; the worker waits `max_backoff` between attempts.
    Unimplemented,

    /// Any other failure; the worker retries with backoff.
    Transport(GenericError),
}

#[async_trait]
impl ConfigSource for RemoteAgentClient {
    async fn get_configs(&mut self, _request: ClientGetConfigsRequest) -> Result<ClientGetConfigsResponse, FetchError> {
        // TODO: forward to `RemoteAgentClient::client_get_configs`, mapping `Code::Unimplemented` to
        // `FetchError::Unimplemented` and every other status to `FetchError::Transport`.
        todo!()
    }
}
