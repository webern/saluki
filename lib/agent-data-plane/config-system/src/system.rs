//! Configuration system lifecycle types.

use agent_data_plane_config::{BootstrapConfiguration, SalukiConfiguration};

use crate::{bootstrap::BootstrapInputs, datadog_agent::DatadogAgentConnection, stream::ConfigStreamHandle};

/// Coordinates bootstrap loading, authority resolution, and translation.
#[derive(Clone, Debug)]
pub struct ConfigurationSystem {
    /// TODO: figure out the actual struct fields needed.
    pub inputs: BootstrapInputs,
}

/// Result of starting the configuration system.
///
/// This is the typed boundary returned to the binary after raw local sources have been consumed,
/// runtime authority has been resolved, and source configuration has been translated.
#[derive(Clone, Debug)]
pub struct StartedConfigurationSystem {
    bootstrap: BootstrapConfiguration,
    saluki: SalukiConfiguration,
    attachments: StartedAttachments,
}

impl StartedConfigurationSystem {
    /// Returns the typed bootstrap configuration.
    pub const fn bootstrap(&self) -> &BootstrapConfiguration {
        &self.bootstrap
    }

    /// Returns the ADP-native runtime configuration.
    pub const fn saluki(&self) -> &SalukiConfiguration {
        &self.saluki
    }

    /// Returns the provider attachments created during startup.
    pub const fn attachments(&self) -> &StartedAttachments {
        &self.attachments
    }
}

/// Provider attachments created by the selected runtime authority.
#[derive(Clone, Debug)]
pub enum StartedAttachments {
    /// No long-lived provider attachment was created.
    None,

    /// Datadog Agent config stream authority is active.
    DatadogAgentConfigStream {
        /// Datadog Agent connection/session capability.
        connection: DatadogAgentConnection,

        /// Runtime configuration stream handle.
        stream: ConfigStreamHandle,
    },
}
