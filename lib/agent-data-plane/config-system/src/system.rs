//! Configuration system lifecycle types.

use std::collections::HashSet;

use agent_data_plane_config::{
    BootstrapConfiguration, BootstrapStartupConfiguration, BootstrapTelemetryConfiguration, ConfigStreamAuthority,
    DataPlaneConfiguration, OtlpPipelineConfiguration, OtlpProxyConfiguration, PipelineConfiguration,
    RuntimeConfigAuthority, RuntimeConfigLanguage, SalukiConfiguration,
};
use datadog_agent_commons::ipc::config::RemoteAgentClientConfiguration;
use datadog_agent_config::classifier::{ConfigClassifier, Pipeline, PipelineAffinity, Severity, SupportLevel};
use saluki_config::{ConfigurationLoader, GenericConfiguration};
use saluki_error::{generic_error, ErrorContext as _, GenericError};
use saluki_io::net::{GrpcTargetAddress, ListenAddress};
use tracing::{debug, error, info, trace, warn};

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

    let saluki = SalukiConfiguration {
        data_plane: DataPlaneConfiguration::new(
            config
                .try_get_typed("data_plane.enabled")
                .error_context("Failed to read `data_plane.enabled`.")?
                .unwrap_or(false),
            checks,
            dogstatsd,
            otlp,
        ),
    };

    let active_pipelines = active_pipelines(&saluki.data_plane);
    check_and_warn_config(config, &active_pipelines).error_context("Incompatible configuration detected.")?;

    Ok(saluki)
}

fn active_pipelines(dp_config: &DataPlaneConfiguration) -> HashSet<Pipeline> {
    let mut s = HashSet::new();
    if dp_config.dogstatsd().enabled() {
        s.insert(Pipeline::DogStatsD);
    }
    if dp_config.checks().enabled() {
        s.insert(Pipeline::Checks);
    }
    if dp_config.otlp().enabled() {
        s.insert(Pipeline::Otlp);
    }
    if dp_config.traces_pipeline_required() {
        s.insert(Pipeline::Traces);
    }
    s
}

fn check_and_warn_config(
    config: &GenericConfiguration, active_pipelines: &HashSet<Pipeline>,
) -> Result<(), GenericError> {
    let classifier = ConfigClassifier::new();
    let mut high_severity_incompatibilities = 0u32;
    debug!("Analyzing configuration.");
    for (key, val) in config
        .flattened_keys()
        .error_context("Unable to flatten configuration into a list of dot-separated keys.")?
    {
        let Some(classification) = classifier.classify(&key, &val) else {
            continue;
        };

        if !is_a_pipeline_affected(active_pipelines, &classification.pipeline_affinity) {
            continue;
        }

        if classification.is_default {
            trace!(key = %key, "Configuration key has a default value.");
            continue;
        }

        match classification.support_level {
            SupportLevel::Incompatible(Severity::Low) => debug!("Low-severity incompatible key detected. Proceeding."),
            SupportLevel::Partial => {
                warn!(key = %key, "Partially supported configuration key. See documentation for details. Proceeding.")
            }
            SupportLevel::Incompatible(Severity::Medium) => {
                warn!(key = %key, "Unsupported configuration key. Proceeding.")
            }
            SupportLevel::Incompatible(Severity::High) => {
                error!(key = %key, "Unsupported configuration key with non-default value. ADP cannot run safely with this setting.");
                high_severity_incompatibilities += 1;
            }
            SupportLevel::Ignored | SupportLevel::Unrecognized => {
                trace!(key = %key, "Configuration key not-applicable. Silently ignoring.")
            }
        }
    }

    if high_severity_incompatibilities > 0 {
        return Err(generic_error!(
            "{high_severity_incompatibilities} incompatible configuration detected. ADP cannot start. Review error logs for details."
        ));
    }

    Ok(())
}

fn is_a_pipeline_affected(active_pipelines: &HashSet<Pipeline>, pipeline_affinity: &PipelineAffinity) -> bool {
    match pipeline_affinity {
        PipelineAffinity::Pipelines(affected_pipelines) => {
            for affected_pipeline in *affected_pipelines {
                if active_pipelines.contains(affected_pipeline) {
                    return true;
                }
            }
            false
        }
        PipelineAffinity::CrossCutting => true,
    }
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
