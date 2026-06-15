//! Datadog Agent attachment used by streamed configuration authority.

use datadog_agent_commons::ipc::session::SessionIdHandle;

/// Long-lived Datadog Agent attachment established by the configuration system.
///
/// This owns the client/session capability needed by the Datadog config stream and by ADP's
/// Datadog Agent integrations. It replaces the config-authority half of the old bin-local
/// remote-agent bootstrap object.
#[derive(Clone, Debug)]
pub struct DatadogAgentConnection {
    /// TODO: figure out the actual struct fields needed.
    pub session_id: SessionIdHandle,

    /// TODO: figure out the actual struct fields needed.
    pub service_names: Vec<String>,
}
