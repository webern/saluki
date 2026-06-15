//! Configuration system lifecycle types.

use agent_data_plane_config::{
    BootstrapConfiguration, BootstrapStartupConfiguration, BootstrapTelemetryConfiguration, ConfigStreamAuthority,
    DataPlaneConfiguration, OtlpPipelineConfiguration, OtlpProxyConfiguration, PipelineConfiguration,
    RuntimeConfigAuthority, RuntimeConfigLanguage, SalukiConfiguration,
};
use saluki_config::{ConfigurationLoader, GenericConfiguration};
use saluki_error::{ErrorContext as _, GenericError};

use crate::{bootstrap::BootstrapInputs, datadog_agent::DatadogAgentConnection, stream::ConfigStreamHandle};

/// Coordinates bootstrap loading, authority resolution, and translation.
#[derive(Clone, Debug)]
pub struct ConfigurationSystem {
    /// Process-local startup inputs.
    pub inputs: BootstrapInputs,
}

impl ConfigurationSystem {
    /// Starts the configuration system from the configured local sources.
    pub async fn start(self) -> Result<StartedConfigurationSystem, GenericError> {
        let local = load_local_datadog_sources(&self.inputs).await?;
        start_from_local_datadog_snapshot(local).await
    }
}

async fn load_local_datadog_sources(inputs: &BootstrapInputs) -> Result<GenericConfiguration, GenericError> {
    Ok(ConfigurationLoader::default()
        .from_yaml(&inputs.config_file_path)
        .error_context("Failed to load Datadog Agent configuration file during configuration-system bootstrap.")?
        .from_environment(inputs.env_var_prefix)
        .error_context("Environment variable prefix should not be empty.")?
        .bootstrap_generic())
}

async fn start_from_local_datadog_snapshot(
    config: GenericConfiguration,
) -> Result<StartedConfigurationSystem, GenericError> {
    let bootstrap = translate_bootstrap_configuration(&config)?;
    let saluki = translate_datadog_snapshot(&config)?;

    Ok(StartedConfigurationSystem {
        bootstrap,
        saluki,
        attachments: StartedAttachments::None,
    })
}

fn translate_bootstrap_configuration(config: &GenericConfiguration) -> Result<BootstrapConfiguration, GenericError> {
    let standalone = config
        .try_get_typed("data_plane.standalone_mode")
        .error_context("Failed to read `data_plane.standalone_mode`.")?
        .unwrap_or(false);
    let remote_agent_enabled = config
        .try_get_typed("data_plane.remote_agent_enabled")
        .error_context("Failed to read `data_plane.remote_agent_enabled`.")?
        .unwrap_or(true);
    let use_config_stream = config
        .try_get_typed("data_plane.use_new_config_stream_endpoint")
        .error_context("Failed to read `data_plane.use_new_config_stream_endpoint`.")?
        .unwrap_or(true);

    let runtime_config_authority = if standalone || !(remote_agent_enabled || use_config_stream) {
        RuntimeConfigAuthority::LocalSnapshot(RuntimeConfigLanguage::DatadogAgent)
    } else {
        RuntimeConfigAuthority::ConfigStream(ConfigStreamAuthority::DatadogAgent)
    };

    let metrics_level = config
        .try_get_typed("metrics_level")
        .error_context("Failed to read `metrics_level`.")?;

    Ok(BootstrapConfiguration {
        startup: BootstrapStartupConfiguration {
            runtime_config_authority,
        },
        telemetry: BootstrapTelemetryConfiguration { metrics_level },
    })
}

fn translate_datadog_snapshot(config: &GenericConfiguration) -> Result<SalukiConfiguration, GenericError> {
    let checks = PipelineConfiguration::new(
        config
            .try_get_typed("data_plane.checks.enabled")
            .error_context("Failed to read `data_plane.checks.enabled`.")?
            .unwrap_or(false),
    );
    let dogstatsd = PipelineConfiguration::new(
        config
            .try_get_typed("data_plane.dogstatsd.enabled")
            .error_context("Failed to read `data_plane.dogstatsd.enabled`.")?
            .unwrap_or(true),
    );
    let otlp_proxy = OtlpProxyConfiguration::new(
        config
            .try_get_typed("data_plane.otlp.proxy.enabled")
            .error_context("Failed to read `data_plane.otlp.proxy.enabled`.")?
            .unwrap_or(false),
        config
            .try_get_typed("data_plane.otlp.proxy.receiver.protocols.grpc.endpoint")
            .error_context("Failed to read `data_plane.otlp.proxy.receiver.protocols.grpc.endpoint`.")?
            .unwrap_or_else(|| "http://localhost:4319".to_string()),
        config
            .try_get_typed("data_plane.otlp.proxy.metrics.enabled")
            .error_context("Failed to read `data_plane.otlp.proxy.metrics.enabled`.")?
            .unwrap_or(true),
        config
            .try_get_typed("data_plane.otlp.proxy.logs.enabled")
            .error_context("Failed to read `data_plane.otlp.proxy.logs.enabled`.")?
            .unwrap_or(true),
        config
            .try_get_typed("data_plane.otlp.proxy.traces.enabled")
            .error_context("Failed to read `data_plane.otlp.proxy.traces.enabled`.")?
            .unwrap_or(true),
    );
    let otlp = OtlpPipelineConfiguration::new(
        config
            .try_get_typed("data_plane.otlp.enabled")
            .error_context("Failed to read `data_plane.otlp.enabled`.")?
            .unwrap_or(false),
        otlp_proxy,
    );

    Ok(SalukiConfiguration {
        data_plane: DataPlaneConfiguration::new(
            config
                .try_get_typed("data_plane.enabled")
                .error_context("Failed to read `data_plane.enabled`.")?
                .unwrap_or(false),
            checks,
            dogstatsd,
            otlp,
        ),
    })
}

/// Result of starting the configuration system.
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

#[cfg(test)]
mod tests {
    use saluki_config::ConfigurationLoader;
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn local_snapshot_translates_pipeline_enablement() {
        let (config, _) = ConfigurationLoader::for_tests(
            Some(json!({
                "data_plane": {
                    "enabled": true,
                    "standalone_mode": true,
                    "checks": { "enabled": true },
                    "dogstatsd": { "enabled": false },
                    "otlp": { "enabled": true }
                }
            })),
            None,
            false,
        )
        .await;

        let started = start_from_local_datadog_snapshot(config)
            .await
            .expect("start configuration system");

        assert_eq!(
            started.bootstrap().startup.runtime_config_authority,
            RuntimeConfigAuthority::LocalSnapshot(RuntimeConfigLanguage::DatadogAgent)
        );
        assert!(started.saluki().data_plane.enabled());
        assert!(started.saluki().data_plane.checks().enabled());
        assert!(!started.saluki().data_plane.dogstatsd().enabled());
        assert!(started.saluki().data_plane.otlp().enabled());
    }

    #[tokio::test]
    async fn stream_authority_is_selected_by_default() {
        let (config, _) = ConfigurationLoader::for_tests(Some(json!({})), None, false).await;

        let started = start_from_local_datadog_snapshot(config)
            .await
            .expect("start configuration system");

        assert_eq!(
            started.bootstrap().startup.runtime_config_authority,
            RuntimeConfigAuthority::ConfigStream(ConfigStreamAuthority::DatadogAgent)
        );
    }
}
