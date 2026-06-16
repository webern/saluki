//! Configuration system lifecycle types.

use std::{
    collections::{HashMap, HashSet},
    num::NonZeroU64,
    path::PathBuf,
    str::FromStr,
    time::Duration,
};

use agent_data_plane_config::{
    AggregateConfiguration, ApmStatsTransformConfiguration, BootstrapConfiguration, BootstrapStartupConfiguration,
    BootstrapTelemetryConfiguration, ChecksIpcConfiguration, ConfigStreamAuthority, ControlPlaneConfiguration,
    DataPlaneConfiguration, DatadogApmStatsEncoderConfiguration, DatadogEventsEncoderConfiguration,
    DatadogForwarderConfiguration, DatadogForwarderEndpointConfiguration, DatadogForwarderHttpProtocol,
    DatadogForwarderRetryConfiguration, DatadogLogsEncoderConfiguration, DatadogMetricsEncoderConfiguration,
    DatadogOpwMetricsConfiguration, DatadogProxyConfiguration, DatadogServiceChecksEncoderConfiguration,
    DatadogTraceEncoderConfiguration, DogStatsDCliConfiguration, DogStatsDDebugLogConfiguration,
    DogStatsDEnablePayloadsConfiguration, DogStatsDMapperConfiguration, DogStatsDMapperProfileConfiguration,
    DogStatsDMetricMappingConfiguration, DogStatsDOriginEnrichmentConfiguration, DogStatsDOriginTagCardinality,
    DogStatsDPostAggregateFilterConfiguration, DogStatsDPrefixFilterConfiguration, DogStatsDSourceConfiguration,
    DynamicValue, EnvironmentConfiguration, MetricTagFilterAction, MetricTagFilterEntry,
    MultiRegionFailoverConfiguration, OtlpForwarderConfiguration, OtlpPipelineConfiguration, OtlpProxyConfiguration,
    OtlpReceiverConfiguration, OtlpSourceConfiguration, OtlpTracesConfiguration, OttlErrorMode,
    OttlFilterConfiguration, OttlTransformConfiguration, PipelineConfiguration, RuntimeConfigAuthority,
    RuntimeConfigLanguage, SalukiConfiguration, TagFilterlistConfiguration, TraceObfuscationConfiguration,
    TraceSamplerConfiguration,
};
use bytesize::ByteSize;
use datadog_agent_commons::{
    ipc::config::{IpcAuthConfiguration, RemoteAgentClientConfiguration},
    platform::PlatformSettings,
};
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
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;
use serde_with::{serde_as, DisplayFromStr, PickFirst};
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

    /// Starts the configuration system from already loaded local Datadog-shaped bootstrap sources.
    pub async fn start_from_loaded_sources(
        self, local: LocalDatadogSources,
    ) -> Result<StartedConfigurationSystem, GenericError> {
        start_from_local_datadog_sources(local.config, &self.inputs).await
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
                resolved_datadog_source: config,
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
                resolved_datadog_source: dynamic_config,
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
        resolved_datadog_source: config,
    })
}

fn dynamic_value_from_key<T>(config: &GenericConfiguration, key: &'static str, initial: T) -> DynamicValue<T>
where
    T: Clone + DeserializeOwned + Send + Sync + 'static,
{
    dynamic_value_from_key_mapped::<T, T, _>(config, key, initial, |value| value)
}

fn dynamic_value_from_key_mapped<S, T, F>(
    config: &GenericConfiguration, key: &'static str, initial: T, map: F,
) -> DynamicValue<T>
where
    S: DeserializeOwned + Send + 'static,
    T: Clone + Send + Sync + 'static,
    F: Fn(S) -> T + Send + Sync + 'static,
{
    let mut watcher = config.watch_for_updates(key);
    let (tx, rx) = watch::channel(initial.clone());

    spawn_traced_named(format!("adp-dynamic-config-{key}"), async move {
        loop {
            let (_, maybe_value) = watcher.changed::<S>().await;
            if let Some(value) = maybe_value {
                if tx.send(map(value)).is_err() {
                    return;
                }
            }
        }
    });

    DynamicValue::new(initial, rx)
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

fn default_metric_prefix_blocklist() -> Vec<String> {
    vec![
        "datadog.agent".to_string(),
        "datadog.dogstatsd".to_string(),
        "datadog.process".to_string(),
        "datadog.trace_agent".to_string(),
        "datadog.tracer".to_string(),
        "activemq".to_string(),
        "activemq_58".to_string(),
        "airflow".to_string(),
        "cassandra".to_string(),
        "confluent".to_string(),
        "hazelcast".to_string(),
        "hive".to_string(),
        "ignite".to_string(),
        "jboss".to_string(),
        "jvm".to_string(),
        "kafka".to_string(),
        "presto".to_string(),
        "sidekiq".to_string(),
        "solr".to_string(),
        "tomcat".to_string(),
        "runtime".to_string(),
    ]
}

fn default_histogram_aggregates() -> Vec<String> {
    vec![
        "max".to_string(),
        "median".to_string(),
        "avg".to_string(),
        "count".to_string(),
    ]
}

fn default_histogram_percentiles() -> Vec<String> {
    vec!["0.95".to_string()]
}

#[derive(Clone, Debug, Default, Deserialize)]
struct SourceDogStatsDPrefixFilterConfiguration {
    #[serde(default, rename = "statsd_metric_namespace")]
    metric_prefix: String,
    #[serde(
        default = "default_metric_prefix_blocklist",
        rename = "statsd_metric_namespace_blocklist",
        alias = "statsd_metric_namespace_blacklist"
    )]
    metric_prefix_blocklist: Vec<String>,
    #[serde(default)]
    metric_filterlist: Vec<String>,
    #[serde(default)]
    metric_filterlist_match_prefix: bool,
    #[serde(default, rename = "statsd_metric_blocklist")]
    metric_blocklist: Vec<String>,
    #[serde(default, rename = "statsd_metric_blocklist_match_prefix")]
    metric_blocklist_match_prefix: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct SourceDogStatsDPostAggregateFilterConfiguration {
    #[serde(default)]
    metric_filterlist: Vec<String>,
    #[serde(default)]
    metric_filterlist_match_prefix: bool,
    #[serde(default, rename = "statsd_metric_blocklist")]
    metric_blocklist: Vec<String>,
    #[serde(default, rename = "statsd_metric_blocklist_match_prefix")]
    metric_blocklist_match_prefix: bool,
    #[serde(default = "default_histogram_aggregates")]
    histogram_aggregates: Vec<String>,
    #[serde(default = "default_histogram_percentiles")]
    histogram_percentiles: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SourceMetricTagFilterAction {
    Include,
    #[default]
    Exclude,
}

impl From<SourceMetricTagFilterAction> for MetricTagFilterAction {
    fn from(value: SourceMetricTagFilterAction) -> Self {
        match value {
            SourceMetricTagFilterAction::Include => Self::Include,
            SourceMetricTagFilterAction::Exclude => Self::Exclude,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct SourceMetricTagFilterEntry {
    metric_name: String,
    #[serde(default)]
    action: SourceMetricTagFilterAction,
    tags: Vec<String>,
}

impl From<SourceMetricTagFilterEntry> for MetricTagFilterEntry {
    fn from(value: SourceMetricTagFilterEntry) -> Self {
        MetricTagFilterEntry::new(value.metric_name, value.action.into(), value.tags)
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
struct SourceTagFilterlistConfiguration {
    #[serde(default, rename = "metric_tag_filterlist")]
    entries: Vec<SourceMetricTagFilterEntry>,
}

const fn default_window_duration_seconds() -> NonZeroU64 {
    NonZeroU64::new(10).expect("not zero")
}

const fn default_primary_flush_interval() -> Duration {
    Duration::from_secs(15)
}

const fn default_context_limit() -> usize {
    1_000_000
}

const fn default_counter_expiry_seconds() -> Option<u64> {
    Some(300)
}

const fn default_passthrough_timestamped_metrics() -> bool {
    true
}

const fn default_passthrough_idle_flush_timeout() -> Duration {
    Duration::from_secs(1)
}

#[derive(Clone, Debug, Deserialize)]
struct SourceAggregateConfiguration {
    #[serde(
        rename = "aggregate_window_duration_seconds",
        default = "default_window_duration_seconds"
    )]
    window_duration_seconds: NonZeroU64,
    #[serde(rename = "aggregate_flush_interval", default = "default_primary_flush_interval")]
    primary_flush_interval: Duration,
    #[serde(rename = "aggregate_context_limit", default = "default_context_limit")]
    context_limit: usize,
    #[serde(
        rename = "aggregate_flush_open_windows",
        alias = "dogstatsd_flush_incomplete_buckets",
        default
    )]
    flush_open_windows: bool,
    #[serde(alias = "dogstatsd_expiry_seconds", default = "default_counter_expiry_seconds")]
    counter_expiry_seconds: Option<u64>,
    #[serde(
        rename = "dogstatsd_no_aggregation_pipeline",
        default = "default_passthrough_timestamped_metrics"
    )]
    passthrough_timestamped_metrics: bool,
    #[serde(
        rename = "aggregate_passthrough_idle_flush_timeout",
        default = "default_passthrough_idle_flush_timeout"
    )]
    passthrough_idle_flush_timeout: Duration,
    #[serde(default = "default_histogram_aggregates")]
    histogram_aggregates: Vec<String>,
    #[serde(default = "default_histogram_percentiles")]
    histogram_percentiles: Vec<String>,
    #[serde(default)]
    histogram_copy_to_distribution: bool,
    #[serde(default)]
    histogram_copy_to_distribution_prefix: String,
}

impl Default for SourceAggregateConfiguration {
    fn default() -> Self {
        Self {
            window_duration_seconds: default_window_duration_seconds(),
            primary_flush_interval: default_primary_flush_interval(),
            context_limit: default_context_limit(),
            flush_open_windows: false,
            counter_expiry_seconds: default_counter_expiry_seconds(),
            passthrough_timestamped_metrics: default_passthrough_timestamped_metrics(),
            passthrough_idle_flush_timeout: default_passthrough_idle_flush_timeout(),
            histogram_aggregates: default_histogram_aggregates(),
            histogram_percentiles: default_histogram_percentiles(),
            histogram_copy_to_distribution: false,
            histogram_copy_to_distribution_prefix: String::new(),
        }
    }
}

const fn default_context_string_interner_size() -> ByteSize {
    ByteSize::kib(64)
}

const fn default_dogstatsd_mapper_cache_size() -> usize {
    1000
}

#[derive(Clone, Debug, Default, Deserialize)]
struct SourceDogStatsDMetricMappingConfiguration {
    #[serde(rename = "match")]
    metric_match: String,
    #[serde(default)]
    match_type: String,
    name: String,
    #[serde(default)]
    tags: HashMap<String, String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct SourceDogStatsDMapperProfileConfiguration {
    name: String,
    prefix: String,
    mappings: Vec<SourceDogStatsDMetricMappingConfiguration>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct SourceDogStatsDMapperProfiles(Vec<SourceDogStatsDMapperProfileConfiguration>);

impl FromStr for SourceDogStatsDMapperProfiles {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str::<Vec<SourceDogStatsDMapperProfileConfiguration>>(s).map(Self)
    }
}

#[serde_as]
#[derive(Clone, Debug, Deserialize)]
struct SourceDogStatsDMapperConfiguration {
    #[serde(
        rename = "dogstatsd_mapper_string_interner_size",
        default = "default_context_string_interner_size"
    )]
    context_string_interner_bytes: ByteSize,
    #[serde(
        rename = "dogstatsd_mapper_cache_size",
        default = "default_dogstatsd_mapper_cache_size"
    )]
    cache_size: usize,
    #[serde_as(as = "PickFirst<(DisplayFromStr, _)>")]
    #[serde(default)]
    dogstatsd_mapper_profiles: SourceDogStatsDMapperProfiles,
}

const fn default_true() -> bool {
    true
}

const fn default_dogstatsd_log_file_max_size() -> ByteSize {
    ByteSize::mb(10)
}

const fn default_dogstatsd_log_file_max_rolls() -> usize {
    3
}

#[derive(Clone, Debug, Deserialize)]
struct SourceDogStatsDDebugLogConfiguration {
    #[serde(rename = "dogstatsd_metrics_stats_enable", default)]
    metrics_stats_enabled: bool,
    #[serde(rename = "dogstatsd_logging_enabled", default = "default_true")]
    logging_enabled: bool,
    #[serde(rename = "dogstatsd_log_file", default)]
    log_file: PathBuf,
    #[serde(
        rename = "dogstatsd_log_file_max_size",
        default = "default_dogstatsd_log_file_max_size"
    )]
    log_file_max_size: ByteSize,
    #[serde(
        rename = "dogstatsd_log_file_max_rolls",
        default = "default_dogstatsd_log_file_max_rolls"
    )]
    log_file_max_rolls: usize,
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

fn translate_dogstatsd_prefix_filter_configuration(
    config: &GenericConfiguration,
) -> Result<DogStatsDPrefixFilterConfiguration, GenericError> {
    let source = config
        .as_typed::<SourceDogStatsDPrefixFilterConfiguration>()
        .error_context("Failed to parse DogStatsD prefix/filter configuration.")?;

    Ok(DogStatsDPrefixFilterConfiguration::new(
        source.metric_prefix,
        source.metric_prefix_blocklist,
        dynamic_value_from_key(config, "metric_filterlist", source.metric_filterlist),
        dynamic_value_from_key(
            config,
            "metric_filterlist_match_prefix",
            source.metric_filterlist_match_prefix,
        ),
        dynamic_value_from_key(config, "statsd_metric_blocklist", source.metric_blocklist),
        dynamic_value_from_key(
            config,
            "statsd_metric_blocklist_match_prefix",
            source.metric_blocklist_match_prefix,
        ),
    ))
}

fn translate_dogstatsd_post_aggregate_filter_configuration(
    config: &GenericConfiguration,
) -> Result<DogStatsDPostAggregateFilterConfiguration, GenericError> {
    let source = config
        .as_typed::<SourceDogStatsDPostAggregateFilterConfiguration>()
        .error_context("Failed to parse DogStatsD post-aggregate filter configuration.")?;

    Ok(DogStatsDPostAggregateFilterConfiguration::new(
        dynamic_value_from_key(config, "metric_filterlist", source.metric_filterlist),
        dynamic_value_from_key(
            config,
            "metric_filterlist_match_prefix",
            source.metric_filterlist_match_prefix,
        ),
        dynamic_value_from_key(config, "statsd_metric_blocklist", source.metric_blocklist),
        dynamic_value_from_key(
            config,
            "statsd_metric_blocklist_match_prefix",
            source.metric_blocklist_match_prefix,
        ),
        source.histogram_aggregates,
        source.histogram_percentiles,
    ))
}

fn translate_tag_filterlist_configuration(
    config: &GenericConfiguration,
) -> Result<TagFilterlistConfiguration, GenericError> {
    let source = config
        .as_typed::<SourceTagFilterlistConfiguration>()
        .error_context("Failed to parse metric tag filterlist configuration.")?;
    let entries = source
        .entries
        .into_iter()
        .map(MetricTagFilterEntry::from)
        .collect::<Vec<_>>();
    let context_cache_capacity = config
        .try_get_typed("data_plane.dogstatsd.aggregator_tag_filter_cache_capacity")
        .error_context("Failed to read `data_plane.dogstatsd.aggregator_tag_filter_cache_capacity`.")?
        .unwrap_or(100_000);

    Ok(TagFilterlistConfiguration::new(
        dynamic_value_from_key_mapped::<Vec<SourceMetricTagFilterEntry>, _, _>(
            config,
            "metric_tag_filterlist",
            entries,
            |entries| entries.into_iter().map(MetricTagFilterEntry::from).collect::<Vec<_>>(),
        ),
        context_cache_capacity,
    ))
}

fn translate_aggregate_configuration(config: &GenericConfiguration) -> Result<AggregateConfiguration, GenericError> {
    let source = config
        .as_typed::<SourceAggregateConfiguration>()
        .error_context("Failed to parse DogStatsD aggregate configuration.")?;

    Ok(AggregateConfiguration::new(
        source.window_duration_seconds,
        source.primary_flush_interval,
        source.context_limit,
        source.flush_open_windows,
        source.counter_expiry_seconds,
        source.passthrough_timestamped_metrics,
        source.passthrough_idle_flush_timeout,
        source.histogram_aggregates,
        source.histogram_percentiles,
        source.histogram_copy_to_distribution,
        source.histogram_copy_to_distribution_prefix,
    ))
}

fn translate_dogstatsd_mapper_configuration(
    config: &GenericConfiguration,
) -> Result<DogStatsDMapperConfiguration, GenericError> {
    let source = config
        .as_typed::<SourceDogStatsDMapperConfiguration>()
        .error_context("Failed to parse DogStatsD mapper configuration.")?;
    let profiles = source
        .dogstatsd_mapper_profiles
        .0
        .into_iter()
        .map(|profile| {
            DogStatsDMapperProfileConfiguration::new(
                profile.name,
                profile.prefix,
                profile
                    .mappings
                    .into_iter()
                    .map(|mapping| {
                        DogStatsDMetricMappingConfiguration::new(
                            mapping.metric_match,
                            mapping.match_type,
                            mapping.name,
                            mapping.tags,
                        )
                    })
                    .collect(),
            )
        })
        .collect();

    Ok(DogStatsDMapperConfiguration::new(
        source.context_string_interner_bytes.as_u64() as usize,
        source.cache_size,
        profiles,
    ))
}

const fn default_target_traces_per_second() -> f64 {
    10.0
}

const fn default_errors_per_second() -> f64 {
    10.0
}

const fn default_sampling_percentage() -> f64 {
    100.0
}

const fn default_error_sampling_enabled() -> bool {
    true
}

const fn default_compute_stats_by_span_kind() -> bool {
    true
}

const fn default_peer_tags_aggregation() -> bool {
    true
}

fn default_trace_env() -> String {
    "none".to_string()
}

const fn default_rare_sampler_tps() -> f64 {
    5.0
}

const fn default_rare_sampler_cooldown_secs() -> f64 {
    300.0
}

const fn default_rare_sampler_cardinality() -> usize {
    200
}

#[derive(Clone, Debug, Deserialize)]
struct SourceProbabilisticSamplerConfiguration {
    #[serde(default)]
    enabled: bool,
    #[serde(default = "default_sampling_percentage")]
    sampling_percentage: f64,
}

impl Default for SourceProbabilisticSamplerConfiguration {
    fn default() -> Self {
        Self {
            enabled: false,
            sampling_percentage: default_sampling_percentage(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct SourceRareSamplerConfiguration {
    #[serde(default = "default_rare_sampler_tps")]
    tps: f64,
    #[serde(default = "default_rare_sampler_cooldown_secs")]
    cooldown: f64,
    #[serde(default = "default_rare_sampler_cardinality")]
    cardinality: usize,
}

impl Default for SourceRareSamplerConfiguration {
    fn default() -> Self {
        Self {
            tps: default_rare_sampler_tps(),
            cooldown: default_rare_sampler_cooldown_secs(),
            cardinality: default_rare_sampler_cardinality(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct SourceApmConfigurationSection {
    #[serde(default = "default_target_traces_per_second")]
    target_traces_per_second: f64,
    #[serde(default = "default_errors_per_second")]
    errors_per_second: f64,
    #[serde(default)]
    probabilistic_sampler: SourceProbabilisticSamplerConfiguration,
    #[serde(default = "default_error_sampling_enabled")]
    error_sampling_enabled: bool,
    #[serde(default = "default_compute_stats_by_span_kind")]
    compute_stats_by_span_kind: bool,
    #[serde(default = "default_peer_tags_aggregation")]
    peer_tags_aggregation: bool,
    #[serde(default)]
    peer_tags: Vec<String>,
    #[serde(default = "default_trace_env")]
    default_env: String,
    #[serde(default)]
    rare_sampler: SourceRareSamplerConfiguration,
}

impl Default for SourceApmConfigurationSection {
    fn default() -> Self {
        Self {
            target_traces_per_second: default_target_traces_per_second(),
            errors_per_second: default_errors_per_second(),
            probabilistic_sampler: SourceProbabilisticSamplerConfiguration::default(),
            error_sampling_enabled: default_error_sampling_enabled(),
            compute_stats_by_span_kind: default_compute_stats_by_span_kind(),
            peer_tags_aggregation: default_peer_tags_aggregation(),
            peer_tags: Vec::new(),
            default_env: default_trace_env(),
            rare_sampler: SourceRareSamplerConfiguration::default(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
struct SourceApmConfiguration {
    #[serde(default)]
    apm_config: SourceApmConfigurationSection,
    #[serde(default, rename = "apm_enable_rare_sampler")]
    enable_rare_sampler: bool,
    #[serde(default, rename = "apm_error_tracking_standalone_enabled")]
    error_tracking_standalone_enabled: bool,
}

#[serde_as]
#[derive(Clone, Debug, Default, Deserialize)]
struct SourceForwarderApiKeys(#[serde_as(as = "serde_with::OneOrMany<_>")] Vec<String>);

#[derive(Clone, Debug, Default, Deserialize)]
struct SourceForwarderAdditionalEndpoints(HashMap<String, SourceForwarderApiKeys>);

impl SourceForwarderAdditionalEndpoints {
    fn into_native(self) -> HashMap<String, Vec<String>> {
        self.0.into_iter().map(|(url, keys)| (url, keys.0)).collect()
    }
}

impl FromStr for SourceForwarderAdditionalEndpoints {
    type Err = serde_json::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(value)
    }
}

const fn default_endpoint_concurrency() -> usize {
    10
}

const fn default_endpoint_concurrency_multiplier() -> usize {
    1
}

const fn default_request_timeout_secs() -> u64 {
    20
}

const fn default_endpoint_buffer_size() -> usize {
    100
}

const fn default_forwarder_connection_reset_interval() -> u64 {
    0
}

const fn default_api_key_validation_interval_mins() -> i64 {
    60
}

fn default_site() -> String {
    "datadoghq.com".to_string()
}

fn default_min_tls_version() -> String {
    "tlsv1.2".to_string()
}

const fn default_request_backoff_factor() -> f64 {
    2.0
}

const fn default_request_backoff_base() -> f64 {
    2.0
}

const fn default_request_backoff_max() -> f64 {
    64.0
}

const fn default_request_recovery_error_decrease_factor() -> u32 {
    2
}

const fn default_storage_max_disk_ratio() -> f64 {
    0.8
}

const fn default_outdated_file_in_days() -> u32 {
    10
}

const fn default_retry_queue_capacity_time_interval_secs() -> u64 {
    15 * 60
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SourceForwarderHttpProtocol {
    #[default]
    Auto,
    Http1,
}

impl From<SourceForwarderHttpProtocol> for DatadogForwarderHttpProtocol {
    fn from(value: SourceForwarderHttpProtocol) -> Self {
        match value {
            SourceForwarderHttpProtocol::Auto => Self::Auto,
            SourceForwarderHttpProtocol::Http1 => Self::Http1,
        }
    }
}

#[serde_as]
#[derive(Clone, Debug, Default, Deserialize)]
struct SourceForwarderEndpointConfiguration {
    #[serde(default)]
    api_key: String,
    #[serde(default = "default_site")]
    site: String,
    #[serde(default, alias = "url")]
    dd_url: Option<String>,
    #[serde_as(as = "PickFirst<(DisplayFromStr, _)>")]
    #[serde(default)]
    additional_endpoints: SourceForwarderAdditionalEndpoints,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct SourceForwarderOpwMetricsConfiguration {
    #[serde(default, rename = "observability_pipelines_worker_metrics_enabled")]
    observability_pipelines_worker_enabled: bool,
    #[serde(default, rename = "observability_pipelines_worker_metrics_url")]
    observability_pipelines_worker_url: String,
    #[serde(default, rename = "vector_metrics_enabled")]
    vector_enabled: bool,
    #[serde(default, rename = "vector_metrics_url")]
    vector_url: String,
}

#[derive(Clone, Debug, Deserialize)]
struct SourceForwarderRetryConfiguration {
    #[serde(default = "default_request_backoff_factor", rename = "forwarder_backoff_factor")]
    backoff_factor: f64,
    #[serde(default = "default_request_backoff_base", rename = "forwarder_backoff_base")]
    backoff_base: f64,
    #[serde(default = "default_request_backoff_max", rename = "forwarder_backoff_max")]
    backoff_max: f64,
    #[serde(
        default = "default_request_recovery_error_decrease_factor",
        rename = "forwarder_recovery_interval"
    )]
    recovery_error_decrease_factor: u32,
    #[serde(default, rename = "forwarder_recovery_reset")]
    recovery_reset: bool,
    #[serde(rename = "forwarder_retry_queue_payloads_max_size")]
    retry_queue_payloads_max_size: Option<u64>,
    #[serde(rename = "forwarder_retry_queue_max_size")]
    retry_queue_max_size: Option<u64>,
    #[serde(default, rename = "forwarder_storage_max_size_in_bytes")]
    storage_max_size_bytes: u64,
    #[serde(default, rename = "forwarder_storage_path")]
    storage_path: PathBuf,
    #[serde(
        default = "default_storage_max_disk_ratio",
        rename = "forwarder_storage_max_disk_ratio"
    )]
    storage_max_disk_ratio: f64,
    #[serde(
        default = "default_outdated_file_in_days",
        rename = "forwarder_outdated_file_in_days"
    )]
    outdated_file_in_days: u32,
    #[serde(
        default = "default_retry_queue_capacity_time_interval_secs",
        rename = "forwarder_retry_queue_capacity_time_interval_sec"
    )]
    capacity_time_interval_secs: u64,
}

impl Default for SourceForwarderRetryConfiguration {
    fn default() -> Self {
        Self {
            backoff_factor: default_request_backoff_factor(),
            backoff_base: default_request_backoff_base(),
            backoff_max: default_request_backoff_max(),
            recovery_error_decrease_factor: default_request_recovery_error_decrease_factor(),
            recovery_reset: false,
            retry_queue_payloads_max_size: None,
            retry_queue_max_size: None,
            storage_max_size_bytes: 0,
            storage_path: PathBuf::new(),
            storage_max_disk_ratio: default_storage_max_disk_ratio(),
            outdated_file_in_days: default_outdated_file_in_days(),
            capacity_time_interval_secs: default_retry_queue_capacity_time_interval_secs(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct SourceForwarderProxyConfiguration {
    #[serde(rename = "proxy_http")]
    http_server: Option<String>,
    #[serde(rename = "proxy_https")]
    https_server: Option<String>,
    #[serde(
        default,
        rename = "proxy_no_proxy",
        deserialize_with = "saluki_config::deserialize_space_separated_or_seq"
    )]
    no_proxy: Vec<String>,
    #[serde(default)]
    no_proxy_nonexact_match: bool,
    #[serde(default)]
    use_proxy_for_cloud_metadata: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct SourceForwarderConfiguration {
    #[serde(
        default = "default_endpoint_concurrency",
        rename = "forwarder_max_concurrent_requests"
    )]
    endpoint_concurrency: usize,
    #[serde(
        default = "default_endpoint_concurrency_multiplier",
        rename = "forwarder_num_workers"
    )]
    endpoint_concurrency_multiplier: usize,
    #[serde(default = "default_request_timeout_secs", rename = "forwarder_timeout")]
    request_timeout_secs: u64,
    #[serde(default = "default_endpoint_buffer_size", rename = "forwarder_high_prio_buffer_size")]
    endpoint_buffer_size: usize,
    #[serde(flatten, default)]
    endpoint: SourceForwarderEndpointConfiguration,
    #[serde(flatten, default)]
    retry: SourceForwarderRetryConfiguration,
    #[serde(flatten)]
    proxy: Option<SourceForwarderProxyConfiguration>,
    #[serde(flatten, default)]
    opw_metrics: SourceForwarderOpwMetricsConfiguration,
    #[serde(default, rename = "forwarder_http_protocol")]
    http_protocol: SourceForwarderHttpProtocol,
    #[serde(
        default = "default_forwarder_connection_reset_interval",
        rename = "forwarder_connection_reset_interval"
    )]
    connection_reset_interval_secs: u64,
    #[serde(default)]
    skip_ssl_validation: bool,
    #[serde(default)]
    sslkeylogfile: String,
    #[serde(default = "default_min_tls_version")]
    min_tls_version: String,
    #[serde(default)]
    allow_arbitrary_tags: bool,
    #[serde(
        default = "default_api_key_validation_interval_mins",
        rename = "forwarder_apikey_validation_interval"
    )]
    api_key_validation_interval_mins: i64,
}

fn retry_queue_payloads_max_size(source: &SourceForwarderRetryConfiguration) -> u64 {
    source
        .retry_queue_payloads_max_size
        .or(source.retry_queue_max_size)
        .unwrap_or(15 * 1024 * 1024)
}

fn fix_forwarder_storage_path(config: &GenericConfiguration, storage_path: PathBuf) -> Result<PathBuf, GenericError> {
    if storage_path.parent().is_some() {
        return Ok(storage_path);
    }

    let Some(mut run_path) = config
        .try_get_typed::<PathBuf>("run_path")
        .error_context("Failed to read `run_path` for default forwarder storage path.")?
    else {
        return Ok(storage_path);
    };
    run_path.push("transactions_to_retry");
    Ok(run_path)
}

fn translate_datadog_forwarder_configuration(
    config: &GenericConfiguration,
) -> Result<DatadogForwarderConfiguration, GenericError> {
    let source = config
        .as_typed::<SourceForwarderConfiguration>()
        .error_context("Failed to parse Datadog forwarder configuration.")?;
    let additional_endpoints = source.endpoint.additional_endpoints.into_native();
    let secrets_in_use = secrets_in_use(config)?;

    let api_key_validation_interval_mins = if source.api_key_validation_interval_mins <= 0 {
        warn!(
            config_key = "forwarder_apikey_validation_interval",
            fallback_minutes = default_api_key_validation_interval_mins(),
            "Configured API key validation interval is invalid; using default."
        );
        default_api_key_validation_interval_mins() as u64
    } else {
        source.api_key_validation_interval_mins as u64
    };

    Ok(DatadogForwarderConfiguration::new(
        source.endpoint_concurrency,
        source.endpoint_concurrency_multiplier,
        source.request_timeout_secs,
        source.endpoint_buffer_size,
        DatadogForwarderEndpointConfiguration::new(
            dynamic_value_from_key(config, "api_key", source.endpoint.api_key),
            source.endpoint.site,
            source.endpoint.dd_url,
            dynamic_value_from_key_mapped::<SourceForwarderAdditionalEndpoints, HashMap<String, Vec<String>>, _>(
                config,
                "additional_endpoints",
                additional_endpoints,
                SourceForwarderAdditionalEndpoints::into_native,
            ),
        ),
        DatadogForwarderRetryConfiguration::new(
            source.retry.backoff_factor,
            source.retry.backoff_base,
            source.retry.backoff_max,
            source.retry.recovery_error_decrease_factor,
            source.retry.recovery_reset,
            retry_queue_payloads_max_size(&source.retry),
            source.retry.storage_max_size_bytes,
            fix_forwarder_storage_path(config, source.retry.storage_path)?,
            source.retry.storage_max_disk_ratio,
            source.retry.outdated_file_in_days,
            source.retry.capacity_time_interval_secs,
            DynamicValue::fixed(secrets_in_use),
        ),
        source.proxy.map(|proxy| {
            DatadogProxyConfiguration::new(
                proxy.http_server,
                proxy.https_server,
                proxy.no_proxy,
                proxy.no_proxy_nonexact_match,
                proxy.use_proxy_for_cloud_metadata,
            )
        }),
        DatadogOpwMetricsConfiguration::new(
            source.opw_metrics.observability_pipelines_worker_enabled,
            source.opw_metrics.observability_pipelines_worker_url,
            source.opw_metrics.vector_enabled,
            source.opw_metrics.vector_url,
        ),
        source.http_protocol.into(),
        source.connection_reset_interval_secs,
        source.skip_ssl_validation,
        Some(source.sslkeylogfile.trim().to_string()).filter(|path| !path.is_empty()),
        source.min_tls_version,
        source.allow_arbitrary_tags,
        api_key_validation_interval_mins,
    ))
}

fn secrets_in_use(config: &GenericConfiguration) -> Result<bool, GenericError> {
    let refresh_interval = config
        .try_get_typed::<u64>("secret_refresh_on_api_key_failure_interval")
        .error_context("Failed to read `secret_refresh_on_api_key_failure_interval`.")?
        .unwrap_or_default();
    let backend_command = config
        .try_get_typed::<String>("secret_backend_command")
        .error_context("Failed to read `secret_backend_command`.")?
        .unwrap_or_default();
    Ok(refresh_interval > 0 || !backend_command.trim().is_empty())
}

#[derive(Clone, Debug, Default, Deserialize)]
struct SourceTraceObfuscationConfiguration {
    #[serde(default, rename = "apm_obfuscation_credit_cards_enabled")]
    credit_cards_enabled: bool,
    #[serde(default, rename = "apm_obfuscation_credit_cards_luhn")]
    credit_cards_luhn: bool,
    #[serde(
        default,
        deserialize_with = "saluki_config::deserialize_space_separated_or_seq",
        rename = "apm_obfuscation_credit_cards_keep_values"
    )]
    credit_cards_keep_values: Vec<String>,
    #[serde(default, rename = "apm_obfuscation_http_remove_query_string")]
    http_remove_query_string: bool,
    #[serde(default, rename = "apm_obfuscation_http_remove_paths_with_digits")]
    http_remove_paths_with_digits: bool,
    #[serde(default, rename = "apm_obfuscation_memcached_enabled")]
    memcached_enabled: bool,
    #[serde(default, rename = "apm_obfuscation_memcached_keep_command")]
    memcached_keep_command: bool,
    #[serde(default, rename = "apm_obfuscation_redis_enabled")]
    redis_enabled: bool,
    #[serde(default, rename = "apm_obfuscation_redis_remove_all_args")]
    redis_remove_all_args: bool,
    #[serde(default, rename = "apm_obfuscation_valkey_enabled")]
    valkey_enabled: bool,
    #[serde(default, rename = "apm_obfuscation_valkey_remove_all_args")]
    valkey_remove_all_args: bool,
    #[serde(default, rename = "apm_obfuscation_sql_dbms")]
    sql_dbms: String,
    #[serde(default, rename = "apm_obfuscation_sql_table_names")]
    sql_table_names: bool,
    #[serde(default, rename = "apm_obfuscation_sql_replace_digits")]
    sql_replace_digits: bool,
    #[serde(default, rename = "apm_obfuscation_sql_keep_sql_alias")]
    sql_keep_sql_alias: bool,
    #[serde(default, rename = "apm_obfuscation_sql_dollar_quoted_func")]
    sql_dollar_quoted_func: bool,
    #[serde(default, rename = "apm_obfuscation_mongodb_enabled")]
    mongodb_enabled: bool,
    #[serde(
        default,
        deserialize_with = "saluki_config::deserialize_space_separated_or_seq",
        rename = "apm_obfuscation_mongodb_keep_values"
    )]
    mongodb_keep_values: Vec<String>,
    #[serde(
        default,
        deserialize_with = "saluki_config::deserialize_space_separated_or_seq",
        rename = "apm_obfuscation_mongodb_obfuscate_sql_values"
    )]
    mongodb_obfuscate_sql_values: Vec<String>,
    #[serde(default, rename = "apm_obfuscation_elasticsearch_enabled")]
    elasticsearch_enabled: bool,
    #[serde(
        default,
        deserialize_with = "saluki_config::deserialize_space_separated_or_seq",
        rename = "apm_obfuscation_elasticsearch_keep_values"
    )]
    elasticsearch_keep_values: Vec<String>,
    #[serde(
        default,
        deserialize_with = "saluki_config::deserialize_space_separated_or_seq",
        rename = "apm_obfuscation_elasticsearch_obfuscate_sql_values"
    )]
    elasticsearch_obfuscate_sql_values: Vec<String>,
    #[serde(default, rename = "apm_obfuscation_opensearch_enabled")]
    opensearch_enabled: bool,
    #[serde(
        default,
        deserialize_with = "saluki_config::deserialize_space_separated_or_seq",
        rename = "apm_obfuscation_opensearch_keep_values"
    )]
    opensearch_keep_values: Vec<String>,
    #[serde(
        default,
        deserialize_with = "saluki_config::deserialize_space_separated_or_seq",
        rename = "apm_obfuscation_opensearch_obfuscate_sql_values"
    )]
    opensearch_obfuscate_sql_values: Vec<String>,
}

fn translate_trace_obfuscation_configuration(
    config: &GenericConfiguration,
) -> Result<TraceObfuscationConfiguration, GenericError> {
    let source = config
        .as_typed::<SourceTraceObfuscationConfiguration>()
        .error_context("Failed to parse trace obfuscation configuration.")?;
    Ok(TraceObfuscationConfiguration::new(
        source.credit_cards_enabled,
        source.credit_cards_luhn,
        source.credit_cards_keep_values,
        source.http_remove_query_string,
        source.http_remove_paths_with_digits,
        source.memcached_enabled,
        source.memcached_keep_command,
        source.redis_enabled,
        source.redis_remove_all_args,
        source.valkey_enabled,
        source.valkey_remove_all_args,
        source.sql_dbms,
        source.sql_table_names,
        source.sql_replace_digits,
        source.sql_keep_sql_alias,
        source.sql_dollar_quoted_func,
        source.mongodb_enabled,
        source.mongodb_keep_values,
        source.mongodb_obfuscate_sql_values,
        source.elasticsearch_enabled,
        source.elasticsearch_keep_values,
        source.elasticsearch_obfuscate_sql_values,
        source.opensearch_enabled,
        source.opensearch_keep_values,
        source.opensearch_obfuscate_sql_values,
    ))
}

const fn default_dsd_buffer_size() -> usize {
    8192
}
const fn default_dsd_buffer_count() -> usize {
    128
}
const fn default_dsd_port() -> u16 {
    8125
}
const fn default_dsd_socket_receive_buffer_size() -> usize {
    0
}
const fn default_dsd_tcp_port() -> u16 {
    0
}
const fn default_statsd_forward_port() -> u16 {
    0
}
const fn default_allow_context_heap_allocations() -> bool {
    true
}
const fn default_no_aggregation_pipeline_support() -> bool {
    true
}
const fn default_context_string_interner_entry_count() -> u64 {
    4096
}
const fn default_cached_contexts_limit() -> usize {
    500_000
}
const fn default_cached_tagsets_limit() -> usize {
    500_000
}
const fn default_context_expiry_seconds() -> u64 {
    20
}
const fn default_dsd_permissive_decoding() -> bool {
    true
}
const fn default_dsd_minimum_sample_rate() -> f64 {
    0.000000003845
}
const fn default_dsd_capture_depth() -> usize {
    1024
}
fn default_dsd_tag_cardinality() -> String {
    "low".to_string()
}

#[derive(Clone, Debug, Deserialize)]
struct SourceDogStatsDEnablePayloadsConfiguration {
    #[serde(default = "default_true")]
    series: bool,
    #[serde(default = "default_true")]
    sketches: bool,
    #[serde(default = "default_true")]
    events: bool,
    #[serde(default = "default_true")]
    service_checks: bool,
}

impl Default for SourceDogStatsDEnablePayloadsConfiguration {
    fn default() -> Self {
        Self {
            series: true,
            sketches: true,
            events: true,
            service_checks: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct SourceDogStatsDOriginEnrichmentConfiguration {
    #[serde(rename = "dogstatsd_origin_detection", default)]
    enabled: bool,
    #[serde(rename = "dogstatsd_entity_id_precedence", default)]
    entity_id_precedence: bool,
    #[serde(rename = "dogstatsd_tag_cardinality", default = "default_dsd_tag_cardinality")]
    tag_cardinality: String,
    #[serde(rename = "origin_detection_unified", default)]
    origin_detection_unified: bool,
    #[serde(rename = "dogstatsd_origin_optout_enabled", default = "default_true")]
    origin_detection_optout: bool,
    #[serde(rename = "dogstatsd_origin_detection_client", default)]
    origin_detection_client: bool,
}

impl Default for SourceDogStatsDOriginEnrichmentConfiguration {
    fn default() -> Self {
        Self {
            enabled: false,
            entity_id_precedence: false,
            tag_cardinality: default_dsd_tag_cardinality(),
            origin_detection_unified: false,
            origin_detection_optout: true,
            origin_detection_client: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct SourceDogStatsDSourceConfiguration {
    #[serde(rename = "dogstatsd_buffer_size", default = "default_dsd_buffer_size")]
    buffer_size: usize,
    #[serde(rename = "dogstatsd_buffer_count", default = "default_dsd_buffer_count")]
    buffer_count: usize,
    #[serde(rename = "dogstatsd_port", default = "default_dsd_port")]
    port: u16,
    #[serde(rename = "dogstatsd_so_rcvbuf", default = "default_dsd_socket_receive_buffer_size")]
    socket_receive_buffer_size: usize,
    #[serde(rename = "dogstatsd_tcp_port", default = "default_dsd_tcp_port")]
    tcp_port: u16,
    #[serde(rename = "statsd_forward_host", default)]
    statsd_forward_host: Option<String>,
    #[serde(rename = "statsd_forward_port", default = "default_statsd_forward_port")]
    statsd_forward_port: u16,
    #[serde(rename = "dogstatsd_socket", default)]
    socket_path: Option<String>,
    #[serde(rename = "dogstatsd_stream_socket", default)]
    socket_stream_path: Option<String>,
    #[serde(rename = "dogstatsd_stream_log_too_big", default)]
    stream_log_too_big: bool,
    #[serde(
        rename = "dogstatsd_eol_required",
        default,
        deserialize_with = "saluki_config::deserialize_space_separated_or_seq"
    )]
    eol_required: Vec<String>,
    #[serde(rename = "bind_host", default)]
    bind_host: Option<String>,
    #[serde(rename = "dogstatsd_non_local_traffic", default)]
    non_local_traffic: bool,
    #[serde(rename = "dogstatsd_autoscale_udp_listeners", default)]
    autoscale_udp_listeners: bool,
    #[serde(
        rename = "dogstatsd_allow_context_heap_allocs",
        default = "default_allow_context_heap_allocations"
    )]
    allow_context_heap_allocations: bool,
    #[serde(
        rename = "dogstatsd_no_aggregation_pipeline",
        default = "default_no_aggregation_pipeline_support"
    )]
    no_aggregation_pipeline_support: bool,
    #[serde(
        rename = "dogstatsd_string_interner_size",
        default = "default_context_string_interner_entry_count"
    )]
    context_string_interner_entry_count: u64,
    #[serde(rename = "dogstatsd_string_interner_size_bytes", default)]
    context_string_interner_size_bytes: Option<ByteSize>,
    #[serde(
        rename = "dogstatsd_cached_contexts_limit",
        default = "default_cached_contexts_limit"
    )]
    cached_contexts_limit: usize,
    #[serde(rename = "dogstatsd_cached_tagsets_limit", default = "default_cached_tagsets_limit")]
    cached_tagsets_limit: usize,
    #[serde(
        rename = "dogstatsd_context_expiry_seconds",
        default = "default_context_expiry_seconds"
    )]
    context_expiry_seconds: u64,
    #[serde(
        rename = "dogstatsd_permissive_decoding",
        default = "default_dsd_permissive_decoding"
    )]
    permissive_decoding: bool,
    #[serde(
        rename = "dogstatsd_minimum_sample_rate",
        default = "default_dsd_minimum_sample_rate"
    )]
    minimum_sample_rate: f64,
    #[serde(rename = "enable_payloads", default)]
    enable_payloads: SourceDogStatsDEnablePayloadsConfiguration,
    #[serde(flatten, default)]
    origin_enrichment: SourceDogStatsDOriginEnrichmentConfiguration,
    #[serde(rename = "dogstatsd_tags", default)]
    additional_tags: Vec<String>,
    #[serde(rename = "dogstatsd_capture_path", default)]
    capture_path: PathBuf,
    #[serde(rename = "dogstatsd_capture_depth", default = "default_dsd_capture_depth")]
    capture_depth: usize,
    #[serde(default)]
    provider_kind: String,
}

fn clean_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn translate_dsd_origin_tag_cardinality(value: &str) -> DogStatsDOriginTagCardinality {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" => DogStatsDOriginTagCardinality::None,
        "orchestrator" => DogStatsDOriginTagCardinality::Orchestrator,
        "high" => DogStatsDOriginTagCardinality::High,
        _ => DogStatsDOriginTagCardinality::Low,
    }
}

fn fix_dsd_capture_path(config: &GenericConfiguration, capture_path: PathBuf) -> Result<PathBuf, GenericError> {
    if capture_path.parent().is_some() {
        return Ok(capture_path);
    }

    let Some(mut run_path) = config
        .try_get_typed::<PathBuf>("run_path")
        .error_context("Failed to read `run_path` for default DogStatsD capture path.")?
    else {
        return Ok(capture_path);
    };
    run_path.push("dsd_capture");
    Ok(run_path)
}

fn translate_dogstatsd_source_configuration(
    config: &GenericConfiguration,
) -> Result<DogStatsDSourceConfiguration, GenericError> {
    let source = config
        .as_typed::<SourceDogStatsDSourceConfiguration>()
        .error_context("Failed to parse DogStatsD source configuration.")?;
    let origin = source.origin_enrichment;

    Ok(DogStatsDSourceConfiguration::new(
        source.buffer_size,
        source.buffer_count,
        source.port,
        source.socket_receive_buffer_size,
        source.tcp_port,
        clean_optional_string(source.statsd_forward_host),
        source.statsd_forward_port,
        clean_optional_string(source.socket_path),
        clean_optional_string(source.socket_stream_path),
        source.stream_log_too_big,
        source.eol_required,
        clean_optional_string(source.bind_host),
        source.non_local_traffic,
        source.autoscale_udp_listeners,
        source.allow_context_heap_allocations,
        source.no_aggregation_pipeline_support,
        source.context_string_interner_entry_count,
        source.context_string_interner_size_bytes.map(|value| value.as_u64()),
        source.cached_contexts_limit,
        source.cached_tagsets_limit,
        source.context_expiry_seconds,
        source.permissive_decoding,
        source.minimum_sample_rate,
        DogStatsDEnablePayloadsConfiguration::new(
            source.enable_payloads.series,
            source.enable_payloads.sketches,
            source.enable_payloads.events,
            source.enable_payloads.service_checks,
        ),
        DogStatsDOriginEnrichmentConfiguration::new(
            origin.enabled,
            origin.entity_id_precedence,
            translate_dsd_origin_tag_cardinality(&origin.tag_cardinality),
            origin.origin_detection_unified,
            origin.origin_detection_optout,
            origin.origin_detection_client,
        ),
        source.additional_tags,
        fix_dsd_capture_path(config, source.capture_path)?,
        source.capture_depth.max(1024),
        source.provider_kind,
    ))
}

fn normalize_sampling_rate(rate: f64) -> f64 {
    if rate <= 0.0 || rate >= 1.0 {
        1.0
    } else {
        rate
    }
}

fn get_non_empty_string(config: &GenericConfiguration, key: &'static str) -> Result<Option<String>, GenericError> {
    Ok(config
        .try_get_typed::<String>(key)
        .error_context(format!("Failed to read `{}`.", key))?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty()))
}

fn translate_multi_region_failover_configuration(
    config: &GenericConfiguration,
) -> Result<MultiRegionFailoverConfiguration, GenericError> {
    let failover_metrics = config
        .try_get_typed("multi_region_failover.failover_metrics")
        .error_context("Failed to read `multi_region_failover.failover_metrics`.")?
        .unwrap_or(false);
    let metric_allowlist = config
        .try_get_typed("multi_region_failover.metric_allowlist")
        .error_context("Failed to read `multi_region_failover.metric_allowlist`.")?
        .unwrap_or_default();

    Ok(MultiRegionFailoverConfiguration::new(
        config
            .try_get_typed("multi_region_failover.enabled")
            .error_context("Failed to read `multi_region_failover.enabled`.")?
            .unwrap_or(false),
        dynamic_value_from_key(config, "multi_region_failover.failover_metrics", failover_metrics),
        dynamic_value_from_key(config, "multi_region_failover.metric_allowlist", metric_allowlist),
        get_non_empty_string(config, "multi_region_failover.api_key")?,
        get_non_empty_string(config, "multi_region_failover.site")?,
        get_non_empty_string(config, "multi_region_failover.dd_url")?,
    ))
}

fn translate_datadog_trace_encoder_configuration(
    config: &GenericConfiguration, source: &DatadogConfiguration,
) -> Result<DatadogTraceEncoderConfiguration, GenericError> {
    let apm_source = config
        .as_typed::<SourceApmConfiguration>()
        .error_context("Failed to parse Datadog trace encoder APM configuration.")?;
    Ok(DatadogTraceEncoderConfiguration::new(
        source.serializer_compressor_kind.clone(),
        source.serializer_zstd_compressor_level as i32,
        config
            .try_get_typed("flush_timeout_secs")
            .error_context("Failed to read `flush_timeout_secs`.")?
            .unwrap_or(2),
        config
            .try_get_typed("env")
            .error_context("Failed to read `env`.")?
            .unwrap_or_else(|| "none".to_string()),
        apm_source.apm_config.target_traces_per_second,
        apm_source.apm_config.errors_per_second,
        apm_source.error_tracking_standalone_enabled,
        config
            .try_get_typed("otlp_config.traces.ignore_missing_datadog_fields")
            .error_context("Failed to read `otlp_config.traces.ignore_missing_datadog_fields`.")?
            .unwrap_or(false),
        read_otlp_trace_sampling_percentage(config)?,
    ))
}

fn translate_apm_stats_transform_configuration(
    config: &GenericConfiguration,
) -> Result<ApmStatsTransformConfiguration, GenericError> {
    let source = config
        .as_typed::<SourceApmConfiguration>()
        .error_context("Failed to parse APM stats transform configuration.")?;
    Ok(ApmStatsTransformConfiguration::new(
        source.apm_config.compute_stats_by_span_kind,
        source.apm_config.peer_tags_aggregation,
        source.apm_config.peer_tags,
        source.apm_config.default_env,
        String::new(),
    ))
}

fn translate_trace_sampler_configuration(
    config: &GenericConfiguration,
) -> Result<TraceSamplerConfiguration, GenericError> {
    let source = config
        .as_typed::<SourceApmConfiguration>()
        .error_context("Failed to parse trace sampler configuration.")?;
    let otlp_sampling_rate = normalize_sampling_rate(read_otlp_trace_sampling_percentage(config)? / 100.0);

    Ok(TraceSamplerConfiguration::new(
        source.apm_config.target_traces_per_second,
        source.apm_config.errors_per_second,
        source.apm_config.probabilistic_sampler.enabled,
        source.apm_config.probabilistic_sampler.sampling_percentage,
        source.apm_config.error_sampling_enabled,
        source.error_tracking_standalone_enabled,
        source.enable_rare_sampler,
        source.apm_config.rare_sampler.tps,
        source.apm_config.rare_sampler.cooldown,
        source.apm_config.rare_sampler.cardinality,
        source.apm_config.default_env,
        otlp_sampling_rate,
    ))
}

fn translate_datadog_apm_stats_encoder_configuration(
    config: &GenericConfiguration,
) -> Result<DatadogApmStatsEncoderConfiguration, GenericError> {
    Ok(DatadogApmStatsEncoderConfiguration::new(
        config
            .try_get_typed("flush_timeout_secs")
            .error_context("Failed to read `flush_timeout_secs`.")?
            .unwrap_or(2),
        config
            .try_get_typed("env")
            .error_context("Failed to read `env`.")?
            .unwrap_or_else(|| "none".to_string()),
    ))
}

fn translate_datadog_metrics_encoder_configuration(
    config: &GenericConfiguration, source: &DatadogConfiguration,
) -> Result<DatadogMetricsEncoderConfiguration, GenericError> {
    Ok(DatadogMetricsEncoderConfiguration::new(
        config
            .try_get_typed("serializer_max_metrics_per_payload")
            .error_context("Failed to read `serializer_max_metrics_per_payload`.")?
            .unwrap_or(10_000),
        source.serializer_max_payload_size as usize,
        source.serializer_max_uncompressed_payload_size as usize,
        source.serializer_max_series_payload_size as usize,
        source.serializer_max_series_uncompressed_payload_size as usize,
        source.serializer_max_series_points_per_payload as usize,
        config
            .try_get_typed("flush_timeout_secs")
            .error_context("Failed to read `flush_timeout_secs`.")?
            .unwrap_or(2),
        source.serializer_compressor_kind.clone(),
        source.serializer_zstd_compressor_level as i32,
        source.use_v2_api.clone().unwrap_or_default().series,
        source.log_payloads,
    ))
}

fn translate_dogstatsd_debug_log_configuration(
    config: &GenericConfiguration,
) -> Result<DogStatsDDebugLogConfiguration, GenericError> {
    let source = config
        .as_typed::<SourceDogStatsDDebugLogConfiguration>()
        .error_context("Failed to parse DogStatsD debug log configuration.")?;
    let log_file = if source.log_file.as_os_str().is_empty() {
        PlatformSettings::get_default_dogstatsd_log_file_path()
    } else {
        source.log_file
    };

    if log_file.to_str().is_none() {
        return Err(generic_error!(
            "dogstatsd_log_file must be valid UTF-8, got '{}'",
            log_file.display()
        ));
    }

    Ok(DogStatsDDebugLogConfiguration::new(
        dynamic_value_from_key(config, "dogstatsd_metrics_stats_enable", source.metrics_stats_enabled),
        source.logging_enabled,
        log_file,
        source.log_file_max_size.as_u64(),
        source.log_file_max_rolls,
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
    let datadog_metrics_encoder = translate_datadog_metrics_encoder_configuration(config, &source)?;
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
        source.serializer_compressor_kind.clone(),
        source.serializer_zstd_compressor_level as i32,
        source.log_payloads,
    );
    let datadog_trace_encoder = translate_datadog_trace_encoder_configuration(config, &source)?;
    let datadog_forwarder = translate_datadog_forwarder_configuration(config)?;
    let datadog_apm_stats_encoder = translate_datadog_apm_stats_encoder_configuration(config)?;
    let apm_stats_transform = translate_apm_stats_transform_configuration(config)?;
    let trace_sampler = translate_trace_sampler_configuration(config)?;
    let trace_obfuscation = translate_trace_obfuscation_configuration(config)?;
    let multi_region_failover = translate_multi_region_failover_configuration(config)?;
    let dogstatsd = translate_dogstatsd_source_configuration(config)?;
    let dogstatsd_prefix_filter = translate_dogstatsd_prefix_filter_configuration(config)?;
    let dogstatsd_mapper = translate_dogstatsd_mapper_configuration(config)?;
    let aggregate = translate_aggregate_configuration(config)?;
    let dogstatsd_debug_log = translate_dogstatsd_debug_log_configuration(config)?;
    let dogstatsd_post_aggregate_filter = translate_dogstatsd_post_aggregate_filter_configuration(config)?;
    let tag_filterlist = translate_tag_filterlist_configuration(config)?;
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
        datadog_metrics_encoder,
        datadog_events_encoder,
        datadog_service_checks_encoder,
        datadog_trace_encoder,
        datadog_forwarder,
        datadog_apm_stats_encoder,
        apm_stats_transform,
        trace_sampler,
        trace_obfuscation,
        multi_region_failover,
        dogstatsd,
        dogstatsd_prefix_filter,
        dogstatsd_mapper,
        aggregate,
        dogstatsd_debug_log,
        dogstatsd_post_aggregate_filter,
        tag_filterlist,
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
    resolved_datadog_source: GenericConfiguration,
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
        LoggingConfigurationTranslator::translate(&self.resolved_datadog_source)
    }

    /// Creates the dynamic log-level worker for the resolved configuration source.
    pub fn dynamic_log_level_worker(&self, controller: LoggingOverrideController) -> DynamicLogLevelWorker {
        DynamicLogLevelWorker::new(&self.resolved_datadog_source, controller)
    }

    /// Returns the resolved memory bounds configuration.
    pub fn memory_bounds_configuration(&self) -> Result<MemoryBoundsConfiguration, GenericError> {
        MemoryBoundsConfiguration::try_from_config(&self.resolved_datadog_source)
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
