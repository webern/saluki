//! Configuration system lifecycle types.

use agent_data_plane_config::{
    BootstrapConfiguration, BootstrapStartupConfiguration, BootstrapTelemetryConfiguration, ConfigStreamAuthority,
    DataPlaneConfiguration, OtlpPipelineConfiguration, OtlpProxyConfiguration, PipelineConfiguration,
    RuntimeConfigAuthority, RuntimeConfigLanguage, SalukiConfiguration,
};
use datadog_agent_commons::ipc::config::RemoteAgentClientConfiguration;
use saluki_config::{ConfigurationLoader, GenericConfiguration};
use saluki_error::{generic_error, ErrorContext as _, GenericError};
use saluki_io::net::{GrpcTargetAddress, ListenAddress};
use tracing::info;

use crate::{
    bootstrap::BootstrapInputs,
    datadog_agent::{remote_agent_service_names, DatadogAgentConnection},
    stream::ConfigStreamHandle,
};

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
        start_from_local_datadog_sources(local, &self.inputs).await
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

async fn start_from_local_datadog_sources(
    config: GenericConfiguration, inputs: &BootstrapInputs,
) -> Result<StartedConfigurationSystem, GenericError> {
    let bootstrap = translate_bootstrap_configuration(&config)?;

    match bootstrap.startup.runtime_config_authority {
        RuntimeConfigAuthority::LocalSnapshot(RuntimeConfigLanguage::DatadogAgent) => {
            let saluki = translate_datadog_snapshot(&config)?;
            Ok(StartedConfigurationSystem {
                bootstrap,
                saluki,
                attachments: StartedAttachments::None,
            })
        }
        RuntimeConfigAuthority::LocalSnapshot(language) => Err(generic_error!(
            "runtime configuration language {:?} is not supported yet",
            language
        )),
        RuntimeConfigAuthority::ConfigStream(ConfigStreamAuthority::DatadogAgent) => {
            let client_config = RemoteAgentClientConfiguration::from_configuration(&config)?;
            let secure_api_listen_address = config
                .try_get_typed("data_plane.secure_api_listen_address")
                .error_context("Failed to read `data_plane.secure_api_listen_address`.")?
                .unwrap_or_else(|| ListenAddress::any_tcp(5101));
            let api_listen_addr =
                GrpcTargetAddress::try_from_listen_addr(&secure_api_listen_address).ok_or_else(|| {
                    generic_error!("Failed to get valid gRPC target address from secure API listen address.")
                })?;

            let connection = DatadogAgentConnection::connect_and_register(
                client_config,
                api_listen_addr,
                remote_agent_service_names(),
            )
            .await?;
            let stream = ConfigStreamHandle::new(ConfigStreamAuthority::DatadogAgent, false);
            let dynamic_config = ConfigurationLoader::default()
                .from_environment(inputs.env_var_prefix)
                .error_context("Environment variable prefix should not be empty.")?
                .with_dynamic_configuration(connection.create_config_stream())
                .into_generic()
                .await?;

            info!("Waiting for initial configuration from Datadog Agent...");
            dynamic_config.ready().await;
            info!("Initial configuration received.");

            let saluki = translate_datadog_snapshot(&dynamic_config)?;

            Ok(StartedConfigurationSystem {
                bootstrap,
                saluki,
                attachments: StartedAttachments::DatadogAgentConfigStream {
                    connection,
                    stream: stream.with_initial_snapshot_received(true),
                },
            })
        }
    }
}

#[cfg(test)]
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
