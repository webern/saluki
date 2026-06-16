//! Transitional runtime shell that still owns the raw `GenericConfiguration`.
//!
//! TRANSITIONAL (spike, build-order step 10): the data topology and most subsystems are built from
//! the native [`SalukiConfiguration`] in `runtime.rs`. The subsystems that are *not yet* native —
//! bootstrap/authority resolution, the environment provider (incl. the workload-collector layer that
//! reaches into `saluki-env`), and the internal supervisor (`ConfigWorker`, `DynamicLogLevelWorker`,
//! IPC server TLS) — are encapsulated here so `runtime.rs` consumes only typed outputs.
//!
//! In the end state this is replaced by `ConfigurationSystem::start()` returning the bootstrap
//! config, `SalukiConfiguration`, and a `DatadogAgentConnection` provider attachment, with the
//! env/control-plane subsystems consuming the shared connection. That rewrite (the design's open
//! shared-connection question + the `saluki-env`/`saluki-app` changes) is tracked in
//! `CONFRA_SPIKE_STATUS.md` and `design/spike-2c-claude.md` (D13).

use agent_data_plane_config::SalukiConfiguration;
use datadog_agent_commons::platform::PlatformSettings;
use resource_accounting::ComponentRegistry;
use saluki_app::bootstrap::BootstrapGuard;
use saluki_app::logging::LoggingOverrideController;
use saluki_components::config::{DatadogRemapper, KEY_ALIASES};
use saluki_config::{ConfigurationLoader, GenericConfiguration};
use saluki_core::health::HealthRegistry;
use saluki_core::runtime::Supervisor;
use saluki_error::{ErrorContext as _, GenericError};
use tracing::{info, warn};

use crate::config::DataPlaneConfiguration;
use crate::internal::env::ADPEnvironmentProvider;
use crate::internal::{
    create_internal_supervisor, logging::LoggingConfigurationTranslator, remote_agent::RemoteAgentBootstrap,
    TopologyControlSurfaces,
};

/// The resolved runtime configuration plus the raw-config-backed subsystems not yet made native.
pub struct RuntimeShell {
    config: GenericConfiguration,
    dp_config: DataPlaneConfiguration,
    saluki_config: SalukiConfiguration,
    ra_bootstrap: Option<RemoteAgentBootstrap>,
}

impl RuntimeShell {
    /// Resolves the runtime configuration: chooses standalone vs config-stream authority, connects
    /// and registers as a remote agent when required, waits for the authoritative snapshot, reloads
    /// logging, and translates into [`SalukiConfiguration`].
    ///
    /// Returns `Ok(None)` when ADP is not enabled.
    pub async fn resolve(
        bootstrap_config: GenericConfiguration, bootstrap_guard: &mut BootstrapGuard,
    ) -> Result<Option<Self>, GenericError> {
        let bootstrap_dp_config = DataPlaneConfiguration::from_configuration(&bootstrap_config)
            .error_context("Failed to load data plane configuration.")?;

        let in_standalone_mode = bootstrap_dp_config.standalone_mode();
        let remote_agent_enabled = bootstrap_dp_config.remote_agent_enabled();
        let use_new_config_stream_endpoint = bootstrap_dp_config.use_new_config_stream_endpoint();
        let should_bootstrap_remote_agent =
            !in_standalone_mode && (remote_agent_enabled || use_new_config_stream_endpoint);

        let ra_bootstrap = if should_bootstrap_remote_agent {
            Some(
                RemoteAgentBootstrap::from_configuration(&bootstrap_config, &bootstrap_dp_config)
                    .await
                    .error_context("Failed to bootstrap remote agent state.")?,
            )
        } else {
            None
        };

        let (config, dp_config) = match &ra_bootstrap {
            Some(ra_bootstrap) if use_new_config_stream_endpoint => {
                let dynamic_config = ConfigurationLoader::default()
                    .with_key_aliases(KEY_ALIASES)
                    .add_providers([DatadogRemapper::new()])
                    .from_environment(PlatformSettings::get_env_var_prefix())?
                    .with_dynamic_configuration(ra_bootstrap.create_config_stream())
                    .into_generic()
                    .await?;

                info!("Waiting for initial configuration from Datadog Agent...");
                dynamic_config.ready().await;
                info!("Initial configuration received.");

                match LoggingConfigurationTranslator::translate(&dynamic_config) {
                    Ok(logging_config) => {
                        if let Err(e) = bootstrap_guard.logging_mut().reload(logging_config).await {
                            warn!(error = %e, "Failed to reload logging from Agent configuration; continuing with bootstrap logging settings.");
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to translate logging configuration from Agent; continuing with bootstrap logging settings.")
                    }
                }

                let dynamic_dp_config = DataPlaneConfiguration::from_configuration(&dynamic_config)
                    .error_context("Failed to load data plane configuration.")?;

                (dynamic_config, dynamic_dp_config)
            }
            _ => (bootstrap_config, bootstrap_dp_config),
        };

        if !in_standalone_mode && !dp_config.enabled() {
            info!("Agent Data Plane is not enabled. Exiting.");
            return Ok(None);
        }

        // Overlay/classifier validation runs inside the configuration system before
        // `SalukiConfiguration` is produced.
        let saluki_config = agent_data_plane_config_system::translate_from_generic(
            &config,
            agent_data_plane_config::RuntimeConfigLanguage::DatadogAgent,
        )
        .error_context("Failed to translate configuration into the ADP-native model.")?;

        Ok(Some(Self {
            config,
            dp_config,
            saluki_config,
            ra_bootstrap,
        }))
    }

    /// Returns the ADP-native runtime configuration.
    pub fn saluki(&self) -> &SalukiConfiguration {
        &self.saluki_config
    }

    /// Returns the (transitional) data-plane configuration used for topology gating.
    pub fn dp_config(&self) -> &DataPlaneConfiguration {
        &self.dp_config
    }

    /// Builds the environment provider (and its optional supervisor).
    ///
    /// Transitional: consumes the raw configuration pending the shared `DatadogAgentConnection`
    /// rewrite of the env/workload-collector layer.
    pub async fn build_environment(
        &self, component_registry: &ComponentRegistry, health_registry: &HealthRegistry,
    ) -> Result<(ADPEnvironmentProvider, Option<Supervisor>), GenericError> {
        ADPEnvironmentProvider::from_configuration(&self.config, &self.dp_config, component_registry, health_registry)
            .await
    }

    /// Builds the internal supervisor (control plane + internal observability).
    ///
    /// Transitional: consumes the raw configuration pending the native control-plane rewrite
    /// (`ConfigWorker`, `DynamicLogLevelWorker`, IPC server TLS) and the Remote Agent service split.
    pub async fn build_internal_supervisor(
        &mut self, component_registry: &ComponentRegistry, health_registry: HealthRegistry,
        control_surfaces: TopologyControlSurfaces, logging_controller: LoggingOverrideController,
    ) -> Result<Supervisor, GenericError> {
        create_internal_supervisor(
            &self.config,
            &self.dp_config,
            component_registry,
            health_registry,
            control_surfaces,
            self.ra_bootstrap.take(),
            logging_controller,
        )
        .await
        .error_context("Failed to create internal supervisor.")
    }
}
