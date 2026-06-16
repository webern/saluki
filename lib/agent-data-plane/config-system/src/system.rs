//! Configuration system lifecycle types.

use std::{
    collections::{HashMap, HashSet},
    num::NonZeroU64,
    path::PathBuf,
    str::FromStr,
    time::Duration,
};

use agent_data_plane_config::{
    AggregateConfiguration, BootstrapConfiguration, BootstrapStartupConfiguration, BootstrapTelemetryConfiguration,
    ChecksIpcConfiguration, ConfigStreamAuthority, ControlPlaneConfiguration, DataPlaneConfiguration,
    DatadogApmStatsEncoderConfiguration, DatadogEventsEncoderConfiguration, DatadogLogsEncoderConfiguration,
    DatadogMetricsEncoderConfiguration, DatadogServiceChecksEncoderConfiguration, DogStatsDCliConfiguration,
    DogStatsDDebugLogConfiguration, DogStatsDMapperConfiguration, DogStatsDMapperProfileConfiguration,
    DogStatsDMetricMappingConfiguration, DogStatsDPostAggregateFilterConfiguration, DogStatsDPrefixFilterConfiguration,
    DynamicValue, EnvironmentConfiguration, MetricTagFilterAction, MetricTagFilterEntry, OtlpForwarderConfiguration,
    OtlpPipelineConfiguration, OtlpProxyConfiguration, OtlpReceiverConfiguration, OtlpSourceConfiguration,
    OtlpTracesConfiguration, OttlErrorMode, OttlFilterConfiguration, OttlTransformConfiguration, PipelineConfiguration,
    RuntimeConfigAuthority, RuntimeConfigLanguage, SalukiConfiguration, TagFilterlistConfiguration,
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
    topology::RuntimeComponentConfiguration,
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
        source.serializer_compressor_kind,
        source.serializer_zstd_compressor_level as i32,
        source.log_payloads,
    );
    let datadog_apm_stats_encoder = translate_datadog_apm_stats_encoder_configuration(config)?;
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
        datadog_apm_stats_encoder,
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

    /// Returns runtime component configuration adapters for topology pieces not yet translated natively.
    pub fn runtime_component_configuration(&self) -> RuntimeComponentConfiguration {
        RuntimeComponentConfiguration::new(self.resolved_datadog_source.clone())
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
