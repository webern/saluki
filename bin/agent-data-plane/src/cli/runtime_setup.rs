//! Runtime resolution for the `run` command.
//!
//! `RuntimeShell` runs the configuration system's startup seam and holds the resulting typed
//! outputs. It contains no `GenericConfiguration`: `ConfigurationSystem::start()` owns the raw
//! loading entirely, and the binary receives `SalukiConfiguration`, a config snapshot, the IPC
//! certificate path, and the (optional) Datadog Agent connection.

use std::path::PathBuf;

use agent_data_plane_config::SalukiConfiguration;
use agent_data_plane_config_system::{BootstrapInputs, ConfigurationSystem, DynamicConfigHandles, StartedAttachments};
use resource_accounting::ComponentRegistry;
use saluki_app::bootstrap::BootstrapGuard;
use saluki_app::logging::LoggingOverrideController;
use saluki_core::health::HealthRegistry;
use saluki_core::runtime::Supervisor;
use saluki_error::{ErrorContext as _, GenericError};
use tracing::{info, warn};

use crate::internal::env::ADPEnvironmentProvider;
use crate::internal::logging::logging_configuration_from_native;
use crate::internal::remote_agent::RemoteAgentServices;
use crate::internal::{create_internal_supervisor, TopologyControlSurfaces};

/// The resolved runtime configuration and the typed capabilities the binary needs.
///
/// Holds no `GenericConfiguration`: the configuration system owns the raw loading.
pub struct RuntimeShell {
    saluki_config: SalukiConfiguration,
    connection: Option<agent_data_plane_config_system::DatadogAgentConnection>,
    /// Typed, scoped dynamic-configuration handles, present when a config-stream authority is active.
    dynamic_handles: Option<DynamicConfigHandles>,
    /// Resolved configuration snapshot served by the control-plane `/config` endpoint.
    config_snapshot: serde_json::Value,
    /// IPC certificate path used to build the privileged API's server TLS.
    ipc_cert_path: PathBuf,
}

impl RuntimeShell {
    /// Runs the configuration system's startup seam and captures its typed outputs.
    ///
    /// `service_names` are the gRPC services the binary exposes (advertised during remote-agent
    /// registration). Returns `Ok(None)` when ADP is not enabled.
    pub async fn resolve(
        inputs: BootstrapInputs, service_names: Vec<String>, bootstrap_guard: &mut BootstrapGuard,
    ) -> Result<Option<Self>, GenericError> {
        let started = ConfigurationSystem::new(inputs, service_names)
            .start()
            .await
            .error_context("Failed to start the ADP configuration system.")?;

        if !started.saluki().data_plane.enabled() {
            info!("Agent Data Plane is not enabled. Exiting.");
            return Ok(None);
        }

        // Reload logging to reflect the authoritative (native) logging configuration.
        match logging_configuration_from_native(&started.saluki().logging) {
            Ok(logging_config) => {
                if let Err(e) = bootstrap_guard.logging_mut().reload(logging_config).await {
                    warn!(error = %e, "Failed to reload runtime logging configuration; continuing with bootstrap settings.");
                }
            }
            Err(e) => {
                warn!(error = %e, "Failed to build runtime logging configuration; continuing with bootstrap settings.")
            }
        }

        let parts = started.into_parts();
        let (connection, dynamic_handles) = match parts.attachments {
            StartedAttachments::DatadogAgentConfigStream { connection, handles } => (Some(connection), Some(handles)),
            StartedAttachments::None => (None, None),
        };

        Ok(Some(Self {
            saluki_config: parts.saluki,
            connection,
            dynamic_handles,
            config_snapshot: parts.config_snapshot,
            ipc_cert_path: parts.ipc_cert_path,
        }))
    }

    /// Returns the ADP-native runtime configuration.
    pub fn saluki(&self) -> &SalukiConfiguration {
        &self.saluki_config
    }

    /// Returns the typed, scoped dynamic-configuration handles, if a config-stream authority is active.
    pub fn dynamic_handles(&self) -> Option<&DynamicConfigHandles> {
        self.dynamic_handles.as_ref()
    }

    /// Builds the environment provider (and its optional supervisor) from native configuration.
    ///
    /// Host/workload/autodiscovery providers reuse the shared Datadog Agent client from the
    /// connection (the D1 decision); standalone (no connection) uses a fixed host provider.
    pub async fn build_environment(
        &self, component_registry: &ComponentRegistry, health_registry: &HealthRegistry,
    ) -> Result<(ADPEnvironmentProvider, Option<Supervisor>), GenericError> {
        let shared_client = self.connection.as_ref().map(|c| c.client());
        ADPEnvironmentProvider::from_saluki(&self.saluki_config, shared_client, component_registry, health_registry)
            .await
    }

    /// Builds the internal supervisor (control plane + internal observability) from native inputs.
    ///
    /// The Remote Agent gRPC services are built from the connection's session (the service split);
    /// the control plane is fed the config snapshot and IPC certificate path. No raw configuration is
    /// consumed.
    pub async fn build_internal_supervisor(
        &mut self, component_registry: &ComponentRegistry, health_registry: HealthRegistry,
        control_surfaces: TopologyControlSurfaces, logging_controller: LoggingOverrideController,
    ) -> Result<Supervisor, GenericError> {
        let services = match &self.connection {
            Some(connection) => Some(RemoteAgentServices::from_session(connection.session_id()).await),
            None => None,
        };

        create_internal_supervisor(
            self.config_snapshot.clone(),
            self.ipc_cert_path.clone(),
            &self.saluki_config.data_plane,
            component_registry,
            health_registry,
            control_surfaces,
            services,
            logging_controller,
        )
        .await
        .error_context("Failed to create internal supervisor.")
    }
}
