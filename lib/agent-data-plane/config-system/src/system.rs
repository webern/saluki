//! Configuration system lifecycle types.

use std::{collections::HashSet, path::PathBuf};

use agent_data_plane_config::{
    BootstrapConfiguration, BootstrapStartupConfiguration, BootstrapTelemetryConfiguration, ChecksIpcConfiguration,
    ConfigStreamAuthority, ControlPlaneConfiguration, DataPlaneConfiguration, DatadogEventsEncoderConfiguration,
    DatadogLogsEncoderConfiguration, DatadogServiceChecksEncoderConfiguration, DogStatsDCliConfiguration,
    EnvironmentConfiguration, OtlpForwarderConfiguration, OtlpPipelineConfiguration, OtlpProxyConfiguration,
    OtlpReceiverConfiguration, OtlpSourceConfiguration, OtlpTracesConfiguration, OttlErrorMode,
    OttlFilterConfiguration, OttlTransformConfiguration, PipelineConfiguration, RuntimeConfigAuthority,
    RuntimeConfigLanguage, SalukiConfiguration,
};
use bytesize::ByteSize;
use datadog_agent_commons::ipc::config::{IpcAuthConfiguration, RemoteAgentClientConfiguration};
use datadog_agent_config::{
    classifier::{ConfigClassifier, Pipeline, PipelineAffinity, Severity, SupportLevel},
    DatadogConfiguration, DatadogRemapper, KEY_ALIASES,
};
use saluki_app::{
    accounting::MemoryBoundsConfiguration,
    config::ConfigView,
    logging::{LoggingConfiguration, LoggingOverrideController},
};
use saluki_common::task::spawn_traced_named;
use saluki_config::{ConfigurationLoader, DurationString, GenericConfiguration};
use saluki_error::{generic_error, ErrorContext as _, GenericError};
use saluki_io::net::{GrpcTargetAddress, ListenAddress};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::watch;
use tracing::{debug, error, info, trace, warn};

use crate::{
    bootstrap::BootstrapInputs,
    datadog_agent::{remote_agent_service_names, DatadogAgentConnection},
    logging::{DynamicLogLevelWorker, LoggingConfigurationTranslator},
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

    /// Loads Datadog-shaped local bootstrap sources for callers that need bootstrap-phase setup.
    pub async fn load_local_datadog_sources(inputs: &BootstrapInputs) -> Result<LocalDatadogSources, GenericError> {
        load_local_datadog_sources(inputs).await.map(LocalDatadogSources::new)
    }

    /// Translates bootstrap decisions from an already loaded local Datadog-shaped bootstrap snapshot.
    pub fn translate_local_datadog_bootstrap(
        config: &GenericConfiguration,
    ) -> Result<BootstrapConfiguration, GenericError> {
        translate_bootstrap_configuration(config)
    }

    /// Translates logging settings from an already loaded local Datadog-shaped snapshot.
    pub fn translate_local_datadog_logging(
        config: &GenericConfiguration,
    ) -> Result<LoggingConfiguration, GenericError> {
        LoggingConfigurationTranslator::translate(config)
    }

    /// Translates DogStatsD CLI/debug settings from an already loaded local Datadog-shaped snapshot.
    pub fn translate_local_datadog_dogstatsd_cli(
        config: &GenericConfiguration,
    ) -> Result<DogStatsDCliConfiguration, GenericError> {
        let socket_path = config
            .try_get_typed::<String>("dogstatsd_socket")
            .error_context("Failed to read `dogstatsd_socket`.")?
            .filter(|path| !path.is_empty())
            .map(PathBuf::from);

        Ok(DogStatsDCliConfiguration::new(socket_path))
    }

    /// Translates data-plane API settings from an already loaded local Datadog-shaped snapshot.
    pub fn translate_local_datadog_data_plane(
        config: &GenericConfiguration,
    ) -> Result<DataPlaneConfiguration, GenericError> {
        translate_data_plane_configuration(config)
    }

    /// Starts the configuration system from an already loaded local Datadog-shaped bootstrap snapshot.
    pub async fn start_from_local_datadog_sources(
        self, local: GenericConfiguration,
    ) -> Result<StartedConfigurationSystem, GenericError> {
        start_from_local_datadog_sources(local, &self.inputs).await
    }
}

/// Loaded local Datadog-shaped sources used during bootstrap.
#[derive(Clone, Debug)]
pub struct LocalDatadogSources {
    config: GenericConfiguration,
}

impl LocalDatadogSources {
    fn new(config: GenericConfiguration) -> Self {
        Self { config }
    }

    /// Translates bootstrap decisions from this local source snapshot.
    pub fn bootstrap_configuration(&self) -> Result<BootstrapConfiguration, GenericError> {
        ConfigurationSystem::translate_local_datadog_bootstrap(&self.config)
    }

    /// Translates logging settings from this local source snapshot.
    pub fn logging_configuration(&self) -> Result<LoggingConfiguration, GenericError> {
        ConfigurationSystem::translate_local_datadog_logging(&self.config)
    }

    /// Translates data-plane API settings from this local source snapshot.
    pub fn data_plane_configuration(&self) -> Result<DataPlaneConfiguration, GenericError> {
        ConfigurationSystem::translate_local_datadog_data_plane(&self.config)
    }

    /// Translates DogStatsD CLI/debug settings from this local source snapshot.
    pub fn dogstatsd_cli_configuration(&self) -> Result<DogStatsDCliConfiguration, GenericError> {
        ConfigurationSystem::translate_local_datadog_dogstatsd_cli(&self.config)
    }

    /// Consumes this wrapper and returns the underlying source snapshot.
    pub fn into_source(self) -> GenericConfiguration {
        self.config
    }
}

async fn load_local_datadog_sources(inputs: &BootstrapInputs) -> Result<GenericConfiguration, GenericError> {
    Ok(ConfigurationLoader::default()
        .with_key_aliases(KEY_ALIASES)
        .from_yaml(&inputs.config_file_path)
        .error_context("Failed to load Datadog Agent configuration file during configuration-system bootstrap.")?
        .add_providers([DatadogRemapper::new()])
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
            let config_view = build_config_view(&config)?;
            Ok(StartedConfigurationSystem {
                bootstrap,
                saluki,
                config_view,
                attachments: StartedAttachments::None,
                compat_datadog_source: config,
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
                .with_key_aliases(KEY_ALIASES)
                .add_providers([DatadogRemapper::new()])
                .from_environment(inputs.env_var_prefix)
                .error_context("Environment variable prefix should not be empty.")?
                .with_dynamic_configuration(connection.create_config_stream())
                .into_generic()
                .await?;

            info!("Waiting for initial configuration from Datadog Agent...");
            dynamic_config.ready().await;
            info!("Initial configuration received.");

            let saluki = translate_datadog_snapshot(&dynamic_config)?;

            let config_view = build_config_view(&dynamic_config)?;
            Ok(StartedConfigurationSystem {
                bootstrap,
                saluki,
                config_view,
                attachments: StartedAttachments::DatadogAgentConfigStream {
                    connection,
                    stream: stream.with_initial_snapshot_received(true),
                },
                compat_datadog_source: dynamic_config,
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

    let config_view = build_config_view(&config)?;
    Ok(StartedConfigurationSystem {
        bootstrap,
        saluki,
        config_view,
        attachments: StartedAttachments::None,
        compat_datadog_source: config,
    })
}

fn build_config_view(config: &GenericConfiguration) -> Result<ConfigView, GenericError> {
    let initial = config
        .as_typed::<Value>()
        .error_context("Failed to serialize runtime configuration for config API view.")?;
    let (tx, rx) = watch::channel(initial);

    if let Some(mut updates) = config.subscribe_for_updates() {
        let live_config = config.clone();
        spawn_traced_named("adp-config-view-task", async move {
            loop {
                if updates.recv().await.is_err() {
                    return;
                }

                match live_config.as_typed::<Value>() {
                    Ok(value) => {
                        if tx.send(value).is_err() {
                            return;
                        }
                    }
                    Err(e) => warn!(error = %e, "Failed to refresh config API view after configuration update."),
                }
            }
        });
    }

    Ok(ConfigView::new(rx))
}

fn translate_bootstrap_configuration(config: &GenericConfiguration) -> Result<BootstrapConfiguration, GenericError> {
    let source = config
        .as_typed::<DatadogConfiguration>()
        .error_context("Failed to parse Datadog Agent bootstrap configuration.")?;
    let data_plane = source.data_plane.unwrap_or_default();
    let standalone = config
        .try_get_typed("data_plane.standalone_mode")
        .error_context("Failed to read `data_plane.standalone_mode`.")?
        .unwrap_or(false);
    let remote_agent_enabled = data_plane.remote_agent_enabled;
    let use_config_stream = data_plane.use_new_config_stream_endpoint;

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

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SourceOttlErrorMode {
    Ignore,
    Silent,
    #[default]
    Propagate,
}

impl From<SourceOttlErrorMode> for OttlErrorMode {
    fn from(value: SourceOttlErrorMode) -> Self {
        match value {
            SourceOttlErrorMode::Ignore => Self::Ignore,
            SourceOttlErrorMode::Silent => Self::Silent,
            SourceOttlErrorMode::Propagate => Self::Propagate,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceOttlFilterTracesConfiguration {
    #[serde(default)]
    span: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceOttlFilterConfiguration {
    #[serde(default)]
    error_mode: SourceOttlErrorMode,
    #[serde(default)]
    traces: SourceOttlFilterTracesConfiguration,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceOttlTransformConfiguration {
    #[serde(default)]
    error_mode: SourceOttlErrorMode,
    #[serde(default)]
    trace_statements: Vec<String>,
}

fn translate_data_plane_configuration(config: &GenericConfiguration) -> Result<DataPlaneConfiguration, GenericError> {
    let source = config
        .as_typed::<DatadogConfiguration>()
        .error_context("Failed to parse Datadog Agent data-plane configuration.")?;
    let data_plane_source = source.data_plane.unwrap_or_default();
    let otlp_proxy_source = data_plane_source.otlp.and_then(|otlp| otlp.proxy).unwrap_or_default();

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
        otlp_proxy_source.metrics.unwrap_or_default().enabled,
        otlp_proxy_source.logs.unwrap_or_default().enabled,
        otlp_proxy_source.traces.unwrap_or_default().enabled,
    );
    let otlp = OtlpPipelineConfiguration::new(
        config
            .try_get_typed("data_plane.otlp.enabled")
            .error_context("Failed to read `data_plane.otlp.enabled`.")?
            .unwrap_or(false),
        otlp_proxy,
    );

    let api_listen_address = config
        .try_get_typed("data_plane.api_listen_address")
        .error_context("Failed to read `data_plane.api_listen_address`.")?
        .unwrap_or_else(|| ListenAddress::any_tcp(5100));
    let secure_api_listen_address = config
        .try_get_typed("data_plane.secure_api_listen_address")
        .error_context("Failed to read `data_plane.secure_api_listen_address`.")?
        .unwrap_or_else(|| ListenAddress::any_tcp(5101));

    Ok(DataPlaneConfiguration::new(
        config
            .try_get_typed("data_plane.enabled")
            .error_context("Failed to read `data_plane.enabled`.")?
            .unwrap_or(false),
        config
            .try_get_typed("data_plane.standalone_mode")
            .error_context("Failed to read `data_plane.standalone_mode`.")?
            .unwrap_or(false),
        api_listen_address,
        secure_api_listen_address,
        checks,
        dogstatsd,
        otlp,
    ))
}

fn translate_datadog_snapshot(config: &GenericConfiguration) -> Result<SalukiConfiguration, GenericError> {
    let source = config
        .as_typed::<DatadogConfiguration>()
        .error_context("Failed to parse Datadog Agent runtime configuration.")?;
    let data_plane = translate_data_plane_configuration(config)?;
    let control_plane =
        ControlPlaneConfiguration::new(IpcAuthConfiguration::from_configuration(config)?.ipc_cert_file_path());
    let checks_ipc = ChecksIpcConfiguration::new(
        config
            .try_get_typed("checks_ipc_endpoint")
            .error_context("Failed to read `checks_ipc_endpoint`.")?
            .unwrap_or_else(|| ListenAddress::any_tcp(5105)),
    );
    let ottl_filter_source = config
        .try_get_typed::<SourceOttlFilterConfiguration>("ottl_filter_config")
        .error_context("Failed to read `ottl_filter_config`.")?
        .unwrap_or_default();
    let ottl_filter =
        OttlFilterConfiguration::new(ottl_filter_source.error_mode.into(), ottl_filter_source.traces.span);
    let ottl_transform_source = config
        .try_get_typed::<SourceOttlTransformConfiguration>("ottl_transform_config")
        .error_context("Failed to read `ottl_transform_config`.")?
        .unwrap_or_default();
    let ottl_transform = OttlTransformConfiguration::new(
        ottl_transform_source.error_mode.into(),
        ottl_transform_source.trace_statements,
    );
    let otlp_grpc_max_recv_msg_size_mib = config
        .try_get_typed::<u64>("otlp_config.receiver.protocols.grpc.max_recv_msg_size_mib")
        .error_context("Failed to read `otlp_config.receiver.protocols.grpc.max_recv_msg_size_mib`.")?
        .unwrap_or(4);
    let otlp_grpc_max_recv_msg_size_mib = if otlp_grpc_max_recv_msg_size_mib == 0 {
        4
    } else {
        otlp_grpc_max_recv_msg_size_mib
    };
    let otlp_receiver = OtlpReceiverConfiguration::new(
        read_otlp_listen_address(
            config,
            "otlp_config.receiver.protocols.http.endpoint",
            "otlp_config.receiver.protocols.http.transport",
            "0.0.0.0:4318",
            "tcp",
        )?,
        read_otlp_listen_address(
            config,
            "otlp_config.receiver.protocols.grpc.endpoint",
            "otlp_config.receiver.protocols.grpc.transport",
            "0.0.0.0:4317",
            "tcp",
        )?,
        (otlp_grpc_max_recv_msg_size_mib * 1024 * 1024) as usize,
    );
    let otlp_traces = OtlpTracesConfiguration::new(
        config
            .try_get_typed("otlp_config.traces.enabled")
            .error_context("Failed to read `otlp_config.traces.enabled`.")?
            .unwrap_or(true),
        config
            .try_get_typed("otlp_config.traces.ignore_missing_datadog_fields")
            .error_context("Failed to read `otlp_config.traces.ignore_missing_datadog_fields`.")?
            .unwrap_or(false),
        config
            .try_get_typed("otlp_config.traces.enable_otlp_compute_top_level_by_span_kind")
            .error_context("Failed to read `otlp_config.traces.enable_otlp_compute_top_level_by_span_kind`.")?
            .unwrap_or(true),
        read_otlp_trace_sampling_percentage(config)?,
        read_byte_size(config, "otlp_config.traces.string_interner_size", ByteSize::kib(512))?,
        config
            .try_get_typed("otlp_config.traces.internal_port")
            .error_context("Failed to read `otlp_config.traces.internal_port`.")?
            .unwrap_or(5003),
    );
    let otlp_source = OtlpSourceConfiguration::new(
        otlp_receiver.clone(),
        config
            .try_get_typed("otlp_config.metrics.enabled")
            .error_context("Failed to read `otlp_config.metrics.enabled`.")?
            .unwrap_or(true),
        config
            .try_get_typed("otlp_config.logs.enabled")
            .error_context("Failed to read `otlp_config.logs.enabled`.")?
            .unwrap_or(true),
        otlp_traces.clone(),
        read_byte_size(config, "otlp_string_interner_size", ByteSize::mib(2))?,
        config
            .try_get_typed("otlp_cached_contexts_limit")
            .error_context("Failed to read `otlp_cached_contexts_limit`.")?
            .unwrap_or(500_000),
        config
            .try_get_typed("otlp_cached_tagsets_limit")
            .error_context("Failed to read `otlp_cached_tagsets_limit`.")?
            .unwrap_or(500_000),
        config
            .try_get_typed("otlp_allow_context_heap_allocs")
            .error_context("Failed to read `otlp_allow_context_heap_allocs`.")?
            .unwrap_or(true),
    );
    let datadog_logs_encoder = DatadogLogsEncoderConfiguration::new(
        source.serializer_compressor_kind.clone(),
        source.serializer_zstd_compressor_level as i32,
    );
    let datadog_events_encoder = DatadogEventsEncoderConfiguration::new(
        source.serializer_max_payload_size as usize,
        source.serializer_max_uncompressed_payload_size as usize,
        source.serializer_compressor_kind.clone(),
        source.serializer_zstd_compressor_level as i32,
        source.log_payloads,
    );
    let datadog_service_checks_encoder = DatadogServiceChecksEncoderConfiguration::new(
        source.serializer_max_payload_size as usize,
        source.serializer_max_uncompressed_payload_size as usize,
        source.serializer_compressor_kind,
        source.serializer_zstd_compressor_level as i32,
        source.log_payloads,
    );
    let otlp_forwarder = OtlpForwarderConfiguration::new(
        data_plane.otlp().proxy().core_agent_otlp_grpc_endpoint().to_string(),
        config
            .try_get_typed("otlp_config.traces.internal_port")
            .error_context("Failed to read `otlp_config.traces.internal_port`.")?
            .unwrap_or(5003),
    );
    let environment = EnvironmentConfiguration::new(
        config
            .try_get_typed("hostname")
            .error_context("Failed to read `hostname`.")?
            .unwrap_or_default(),
        config
            .try_get_typed::<DurationString>("expected_tags_duration")
            .error_context("Failed to read `expected_tags_duration`.")?
            .map(|ds| ds.as_duration())
            .unwrap_or_default(),
    );

    let saluki = SalukiConfiguration {
        data_plane,
        control_plane,
        checks_ipc,
        ottl_filter,
        ottl_transform,
        datadog_logs_encoder,
        datadog_events_encoder,
        datadog_service_checks_encoder,
        otlp_receiver,
        otlp_source,
        otlp_traces,
        otlp_forwarder,
        environment,
    };

    let active_pipelines = active_pipelines(&saluki.data_plane);
    check_and_warn_config(config, &active_pipelines).error_context("Incompatible configuration detected.")?;

    Ok(saluki)
}

fn read_otlp_listen_address(
    config: &GenericConfiguration, endpoint_key: &'static str, transport_key: &'static str,
    default_endpoint: &'static str, default_transport: &'static str,
) -> Result<ListenAddress, GenericError> {
    let endpoint = config
        .try_get_typed::<String>(endpoint_key)
        .error_context(format!("Failed to read `{}`.", endpoint_key))?
        .unwrap_or_else(|| default_endpoint.to_string());
    let transport = config
        .try_get_typed::<String>(transport_key)
        .error_context(format!("Failed to read `{}`.", transport_key))?
        .unwrap_or_else(|| default_transport.to_string());
    let address = format!("{}://{}", transport, endpoint);

    ListenAddress::try_from(address.as_str()).map_err(|e| {
        generic_error!(
            "Failed to parse OTLP listen address from `{}`/`{}` ({}): {}",
            transport_key,
            endpoint_key,
            address,
            e
        )
    })
}

fn read_byte_size(
    config: &GenericConfiguration, key: &'static str, default_value: ByteSize,
) -> Result<usize, GenericError> {
    Ok(config
        .try_get_typed::<ByteSize>(key)
        .error_context(format!("Failed to read `{}`.", key))?
        .unwrap_or(default_value)
        .as_u64() as usize)
}

fn read_otlp_trace_sampling_percentage(config: &GenericConfiguration) -> Result<f64, GenericError> {
    if let Some(value) = config
        .try_get_typed("otlp_config.traces.probabilistic_sampler.sampling_percentage")
        .error_context("Failed to read `otlp_config.traces.probabilistic_sampler.sampling_percentage`.")?
    {
        return Ok(value);
    }

    Ok(config
        .try_get_typed("otlp_config_traces_probabilistic_sampler_sampling_percentage")
        .error_context("Failed to read `otlp_config_traces_probabilistic_sampler_sampling_percentage`.")?
        .unwrap_or(100.0))
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
    config_view: ConfigView,
    attachments: StartedAttachments,
    compat_datadog_source: GenericConfiguration,
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

    /// Returns the runtime configuration view exposed through the config API.
    pub fn config_view(&self) -> ConfigView {
        self.config_view.clone()
    }

    /// Returns the resolved ADP logging configuration.
    pub fn logging_configuration(&self) -> Result<LoggingConfiguration, GenericError> {
        LoggingConfigurationTranslator::translate(&self.compat_datadog_source)
    }

    /// Creates the dynamic log-level worker for the resolved configuration source.
    pub fn dynamic_log_level_worker(&self, controller: LoggingOverrideController) -> DynamicLogLevelWorker {
        DynamicLogLevelWorker::new(&self.compat_datadog_source, controller)
    }

    /// Returns the resolved memory bounds configuration.
    pub fn memory_bounds_configuration(&self) -> Result<MemoryBoundsConfiguration, GenericError> {
        MemoryBoundsConfiguration::try_from_config(&self.compat_datadog_source)
    }

    /// Returns the provider attachments created during startup.
    pub const fn attachments(&self) -> &StartedAttachments {
        &self.attachments
    }

    /// Returns the Datadog-shaped source snapshot for runtime paths that have not yet been cut over.
    pub const fn compat_datadog_source(&self) -> &GenericConfiguration {
        &self.compat_datadog_source
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
