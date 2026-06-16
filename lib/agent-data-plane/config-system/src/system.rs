//! Configuration system lifecycle.
//!
//! [`ConfigurationSystem::start`] is the single startup seam: it loads local bootstrap sources,
//! parses [`BootstrapConfiguration`], chooses a [`RuntimeConfigAuthority`], resolves the
//! authoritative runtime input (a local snapshot, or a connected config stream), loads Saluki-private
//! supplemental configuration, translates the source language into [`SalukiConfiguration`], and
//! returns typed outputs plus any provider attachments. The binary receives typed outputs and
//! nothing else.

use agent_data_plane_config::{
    BootstrapConfiguration, ConfigStreamAuthority, RuntimeConfigAuthority, RuntimeConfigLanguage, SalukiConfiguration,
    SalukiPrivateConfiguration,
};
use datadog_agent_config::DatadogConfiguration;
use saluki_config::dynamic::ConfigUpdate;
use saluki_error::{generic_error, ErrorContext as _, GenericError};

use crate::bootstrap::{self, BootstrapInputs};
use crate::datadog_agent::DatadogAgentConnection;
use crate::stream::ConfigStreamHandle;
use crate::translate::translate_datadog;

/// Coordinates bootstrap loading, authority resolution, and translation.
#[derive(Clone, Debug)]
pub struct ConfigurationSystem {
    inputs: BootstrapInputs,
    service_names: Vec<String>,
}

impl ConfigurationSystem {
    /// Creates a configuration system.
    ///
    /// `service_names` are the gRPC services the binary will expose and advertise during
    /// remote-agent registration (status, flare, telemetry). The configuration system only forwards
    /// them; the binary implements them.
    pub fn new(inputs: BootstrapInputs, service_names: Vec<String>) -> Self {
        Self { inputs, service_names }
    }

    /// Runs the full startup lifecycle and returns the typed configuration boundary.
    pub async fn start(self) -> Result<StartedConfigurationSystem, GenericError> {
        // Local bootstrap sources: the GenericConfiguration lives and dies in this scope.
        let local = bootstrap::load_local_sources(&self.inputs)?;
        let bootstrap = bootstrap::parse_bootstrap(&local)?;

        let language = bootstrap.startup.runtime_config_authority.language();
        let private = SalukiPrivateConfiguration::for_language(language);

        let (saluki, attachments) = match &bootstrap.startup.runtime_config_authority {
            RuntimeConfigAuthority::LocalSnapshot(RuntimeConfigLanguage::DatadogAgent) => {
                let dd_config: DatadogConfiguration = local
                    .as_typed()
                    .error_context("Failed to parse local Datadog configuration snapshot.")?;
                let gates = bootstrap::read_pipeline_gates(&local);
                let saluki = translate_datadog(&dd_config, &private, gates);
                (saluki, StartedAttachments::None)
            }
            RuntimeConfigAuthority::ConfigStream(ConfigStreamAuthority::DatadogAgent) => {
                let connection = DatadogAgentConnection::connect(
                    &local,
                    &bootstrap.startup.secure_api_listen_address,
                    self.service_names.clone(),
                )
                .await
                .error_context("Failed to establish Datadog Agent connection.")?;

                let mut updates = connection.create_config_stream();
                let snapshot = wait_for_initial_snapshot(&mut updates).await?;

                let dd_config: DatadogConfiguration = serde_json::from_value(snapshot.clone())
                    .map_err(|e| generic_error!("Failed to parse authoritative Datadog snapshot: {e}"))?;
                let gates = bootstrap::read_pipeline_gates_value(&snapshot);
                let saluki = translate_datadog(&dd_config, &private, gates);

                let stream = ConfigStreamHandle::new(ConfigStreamAuthority::DatadogAgent, updates);
                (
                    saluki,
                    StartedAttachments::DatadogAgentConfigStream { connection, stream },
                )
            }
            RuntimeConfigAuthority::LocalSnapshot(language) => {
                return Err(generic_error!(
                    "Runtime config language {language:?} is not yet supported by the configuration system."
                ));
            }
        };

        Ok(StartedConfigurationSystem {
            bootstrap,
            saluki,
            attachments,
        })
    }
}

/// Translate an already-resolved raw configuration into [`SalukiConfiguration`].
///
/// Transitional helper for the binary's incremental cutover: it lets `run.rs` obtain the ADP-native
/// configuration from the configuration it has already resolved, so topology components are built
/// from native slices. In the fully collapsed end state `run.rs` receives `SalukiConfiguration`
/// directly from [`ConfigurationSystem::start`] and this helper is unnecessary.
pub fn translate_from_generic(
    config: &saluki_config::GenericConfiguration, language: RuntimeConfigLanguage,
) -> Result<SalukiConfiguration, GenericError> {
    let dd_config: DatadogConfiguration = config
        .as_typed()
        .error_context("Failed to parse Datadog configuration for translation.")?;
    let gates = crate::bootstrap::read_pipeline_gates(config);
    crate::validate::validate_against_overlay(config, gates)?;
    let private = SalukiPrivateConfiguration::for_language(language);
    Ok(translate_datadog(&dd_config, &private, gates))
}

async fn wait_for_initial_snapshot(
    updates: &mut tokio::sync::mpsc::Receiver<ConfigUpdate>,
) -> Result<serde_json::Value, GenericError> {
    loop {
        match updates.recv().await {
            Some(ConfigUpdate::Snapshot(snapshot)) => return Ok(snapshot),
            // Ignore partial updates that arrive before the first full snapshot.
            Some(ConfigUpdate::Partial { .. }) => continue,
            None => {
                return Err(generic_error!(
                    "Config stream closed before the initial snapshot arrived."
                ))
            }
        }
    }
}

/// The typed boundary returned to the binary after startup.
#[derive(Debug)]
pub struct StartedConfigurationSystem {
    bootstrap: BootstrapConfiguration,
    saluki: SalukiConfiguration,
    attachments: StartedAttachments,
}

impl StartedConfigurationSystem {
    /// Returns the typed bootstrap configuration.
    pub fn bootstrap(&self) -> &BootstrapConfiguration {
        &self.bootstrap
    }

    /// Returns the ADP-native runtime configuration.
    pub fn saluki(&self) -> &SalukiConfiguration {
        &self.saluki
    }

    /// Returns the provider attachments created during startup.
    pub fn attachments(&self) -> &StartedAttachments {
        &self.attachments
    }

    /// Consumes the started system, returning ownership of its parts.
    pub fn into_parts(self) -> (BootstrapConfiguration, SalukiConfiguration, StartedAttachments) {
        (self.bootstrap, self.saluki, self.attachments)
    }
}

/// Provider attachments created by the selected runtime authority.
#[derive(Debug)]
pub enum StartedAttachments {
    /// No long-lived provider attachment was created (local snapshot authority).
    None,

    /// Datadog Agent config stream authority is active.
    DatadogAgentConfigStream {
        /// Datadog Agent connection/session capability.
        connection: DatadogAgentConnection,

        /// Runtime configuration stream handle.
        stream: ConfigStreamHandle,
    },
}
