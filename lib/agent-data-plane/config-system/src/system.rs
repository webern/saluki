//! Configuration system lifecycle.
//!
//! [`ConfigurationSystem::start`] is the single startup seam: it loads local bootstrap sources,
//! parses [`BootstrapConfiguration`], chooses a [`RuntimeConfigAuthority`], resolves the
//! authoritative runtime input (a local snapshot, or a connected config stream), loads Saluki-private
//! supplemental configuration, translates the source language into [`SalukiConfiguration`], and
//! returns typed outputs plus any provider attachments. The binary receives typed outputs and
//! nothing else.

use std::path::PathBuf;

use agent_data_plane_config::{
    BootstrapConfiguration, ConfigStreamAuthority, RuntimeConfigAuthority, RuntimeConfigLanguage, SalukiConfiguration,
    SalukiPrivateConfiguration,
};
use datadog_agent_commons::ipc::config::IpcAuthConfiguration;
use datadog_agent_config::DatadogConfiguration;
use saluki_config::ConfigurationLoader;
use saluki_error::{generic_error, ErrorContext as _, GenericError};

use crate::bootstrap::{self, BootstrapInputs};
use crate::datadog_agent::DatadogAgentConnection;
use crate::dynamic::{ConfigUpdateRouter, DynamicConfigHandles};
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
    ///
    /// The raw `GenericConfiguration` is loaded, translated, and dropped entirely within this method;
    /// the binary receives only typed outputs (`SalukiConfiguration`, the bootstrap config, provider
    /// attachments, a config snapshot for the `/config` endpoint, and the IPC certificate path).
    pub async fn start(self) -> Result<StartedConfigurationSystem, GenericError> {
        // Local bootstrap sources: the GenericConfiguration lives and dies in this scope.
        let local = bootstrap::load_local_sources(&self.inputs)?;
        let bootstrap = bootstrap::parse_bootstrap(&local)?;

        let language = bootstrap.startup.runtime_config_authority.language();

        // The IPC certificate path comes from the bootstrap-phase local configuration.
        let ipc_cert_path = IpcAuthConfiguration::from_configuration(&local)?.ipc_cert_file_path();

        let (saluki, config_snapshot, attachments) = match &bootstrap.startup.runtime_config_authority {
            RuntimeConfigAuthority::LocalSnapshot(RuntimeConfigLanguage::DatadogAgent) => {
                let saluki = translate_from_generic(&local, language)?;
                let snapshot = local.as_typed::<serde_json::Value>().unwrap_or(serde_json::Value::Null);
                (saluki, snapshot, StartedAttachments::None)
            }
            RuntimeConfigAuthority::ConfigStream(ConfigStreamAuthority::DatadogAgent) => {
                let connection = DatadogAgentConnection::connect(
                    &local,
                    &bootstrap.startup.secure_api_listen_address,
                    self.service_names.clone(),
                )
                .await
                .error_context("Failed to establish Datadog Agent connection.")?;

                // Build the authoritative configuration from the Agent's config stream, with local
                // environment variables layered on top for ADP-specific overrides, then translate.
                let dynamic = ConfigurationLoader::default()
                    .from_environment(self.inputs.env_var_prefix)?
                    .with_dynamic_configuration(connection.create_config_stream())
                    .into_generic()
                    .await?;
                dynamic.ready().await;

                let saluki = translate_from_generic(&dynamic, language)?;
                let snapshot = dynamic
                    .as_typed::<serde_json::Value>()
                    .unwrap_or(serde_json::Value::Null);

                // Build the typed, scoped dynamic-update router, seeded with the initial translation
                // and snapshot, and run it on a dedicated config stream. The router re-translates
                // inbound updates and pushes changed slices to per-component handles; the binary
                // receives only the typed handles, never the raw stream.
                let mut private = SalukiPrivateConfiguration::for_language(language);
                private.workload = crate::bootstrap::read_workload_config(&dynamic);
                let (router, handles) = ConfigUpdateRouter::new(&saluki, snapshot.clone(), private);
                let stream =
                    ConfigStreamHandle::new(ConfigStreamAuthority::DatadogAgent, connection.create_config_stream());
                tokio::spawn(router.run(stream.into_updates()));

                (
                    saluki,
                    snapshot,
                    StartedAttachments::DatadogAgentConfigStream { connection, handles },
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
            config_snapshot,
            ipc_cert_path,
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
    let mut private = SalukiPrivateConfiguration::for_language(language);
    private.workload = crate::bootstrap::read_workload_config(config);
    let mut saluki = translate_datadog(&dd_config, &private, gates);
    saluki.memory = crate::bootstrap::read_memory_config(config);
    if let Some(proxy) = crate::bootstrap::read_otlp_proxy(config) {
        saluki.data_plane.otlp_proxy_enabled = true;
        saluki.data_plane.otlp_proxy_traces = proxy.proxy_traces;
        saluki.otlp.config.proxy = Some(proxy);
    }
    Ok(saluki)
}

/// The typed boundary returned to the binary after startup.
#[derive(Debug)]
pub struct StartedConfigurationSystem {
    bootstrap: BootstrapConfiguration,
    saluki: SalukiConfiguration,
    attachments: StartedAttachments,
    config_snapshot: serde_json::Value,
    ipc_cert_path: PathBuf,
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

    /// Returns the resolved configuration snapshot (served by the control-plane `/config` endpoint).
    pub fn config_snapshot(&self) -> &serde_json::Value {
        &self.config_snapshot
    }

    /// Returns the IPC certificate path used to build the privileged API's server TLS.
    pub fn ipc_cert_path(&self) -> &PathBuf {
        &self.ipc_cert_path
    }

    /// Consumes the started system, returning ownership of its parts.
    pub fn into_parts(self) -> StartedParts {
        StartedParts {
            bootstrap: self.bootstrap,
            saluki: self.saluki,
            attachments: self.attachments,
            config_snapshot: self.config_snapshot,
            ipc_cert_path: self.ipc_cert_path,
        }
    }
}

/// Owned parts of a [`StartedConfigurationSystem`].
pub struct StartedParts {
    /// Typed bootstrap configuration.
    pub bootstrap: BootstrapConfiguration,
    /// ADP-native runtime configuration.
    pub saluki: SalukiConfiguration,
    /// Provider attachments created during startup.
    pub attachments: StartedAttachments,
    /// Resolved configuration snapshot for the `/config` endpoint.
    pub config_snapshot: serde_json::Value,
    /// IPC certificate path for the privileged API's server TLS.
    pub ipc_cert_path: PathBuf,
}

/// Provider attachments created by the selected runtime authority.
// This value is constructed once at startup and moved once into the runtime shell; it is never stored
// in bulk or on a hot path, so boxing the larger variant to equalize sizes would add indirection for no
// real benefit.
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum StartedAttachments {
    /// No long-lived provider attachment was created (local snapshot authority).
    None,

    /// Datadog Agent config stream authority is active.
    DatadogAgentConfigStream {
        /// Datadog Agent connection/session capability.
        connection: DatadogAgentConnection,

        /// Typed, scoped dynamic-configuration handles for components.
        handles: DynamicConfigHandles,
    },
}
