//! Datadog Agent attachment used by streamed configuration authority.

use datadog_agent_commons::ipc::{
    client::RemoteAgentClient, config::RemoteAgentClientConfiguration, session::SessionIdHandle,
};
use saluki_error::GenericError;

/// Long-lived Datadog Agent attachment established by the configuration system.
#[derive(Clone)]
pub struct DatadogAgentConnection {
    client: RemoteAgentClient,
    session_id: SessionIdHandle,
    service_names: Vec<String>,
}

impl std::fmt::Debug for DatadogAgentConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatadogAgentConnection")
            .field("session_id", &self.session_id)
            .field("service_names", &self.service_names)
            .finish_non_exhaustive()
    }
}

impl DatadogAgentConnection {
    /// Connects to the Datadog Agent from typed IPC configuration.
    pub async fn connect(
        client_config: RemoteAgentClientConfiguration, session_id: SessionIdHandle, service_names: Vec<String>,
    ) -> Result<Self, GenericError> {
        let client = RemoteAgentClient::connect(client_config).await?;
        Ok(Self {
            client,
            session_id,
            service_names,
        })
    }

    /// Returns a clone of the IPC client capability.
    pub fn client(&self) -> RemoteAgentClient {
        self.client.clone()
    }

    /// Returns the remote-agent session handle.
    pub const fn session_id(&self) -> &SessionIdHandle {
        &self.session_id
    }

    /// Returns the service names registered through this connection.
    pub fn service_names(&self) -> &[String] {
        &self.service_names
    }
}
