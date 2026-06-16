//! Component-native configuration structs shared by ADP translators and components.

use std::{collections::HashMap, future::pending, num::NonZeroU64, path::PathBuf, time::Duration};

use saluki_io::net::ListenAddress;
use tokio::sync::watch;

/// Dynamic component configuration value.
#[derive(Clone, Debug)]
pub struct DynamicValue<T> {
    current: T,
    updates: Option<watch::Receiver<T>>,
}

impl<T> DynamicValue<T>
where
    T: Clone,
{
    /// Creates a fixed dynamic value with no update stream.
    pub fn fixed(current: T) -> Self {
        Self { current, updates: None }
    }

    /// Creates a dynamic value from an initial value and update receiver.
    pub fn new(current: T, updates: watch::Receiver<T>) -> Self {
        Self {
            current,
            updates: Some(updates),
        }
    }

    /// Returns the current value.
    pub fn current(&self) -> T {
        self.updates
            .as_ref()
            .map(|updates| updates.borrow().clone())
            .unwrap_or_else(|| self.current.clone())
    }

    /// Waits for the next update, returning `None` only if the update stream closes.
    pub async fn changed(&mut self) -> Option<T> {
        let Some(updates) = self.updates.as_mut() else {
            pending::<()>().await;
            unreachable!("pending future never completes")
        };

        if updates.changed().await.is_err() {
            self.updates = None;
            return None;
        }

        let value = updates.borrow().clone();
        self.current = value.clone();
        Some(value)
    }
}

impl<T> Default for DynamicValue<T>
where
    T: Clone + Default,
{
    fn default() -> Self {
        Self::fixed(T::default())
    }
}

impl<T> PartialEq for DynamicValue<T>
where
    T: Clone + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.current() == other.current()
    }
}

impl<T> Eq for DynamicValue<T> where T: Clone + Eq {}

/// OTTL condition/statement error handling mode.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OttlErrorMode {
    /// Ignore errors and log them.
    Ignore,
    /// Ignore errors without logging.
    Silent,
    /// Treat errors as payload drops.
    #[default]
    Propagate,
}

/// Native OTTL filter settings.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OttlFilterConfiguration {
    error_mode: OttlErrorMode,
    span_conditions: Vec<String>,
}

impl OttlFilterConfiguration {
    /// Creates OTTL filter settings.
    pub fn new(error_mode: OttlErrorMode, span_conditions: Vec<String>) -> Self {
        Self {
            error_mode,
            span_conditions,
        }
    }

    /// Returns the configured error mode.
    pub const fn error_mode(&self) -> OttlErrorMode {
        self.error_mode
    }

    /// Returns configured span filter conditions.
    pub fn span_conditions(&self) -> &[String] {
        &self.span_conditions
    }
}

/// Native OTTL transform settings.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OttlTransformConfiguration {
    error_mode: OttlErrorMode,
    trace_statements: Vec<String>,
}

impl OttlTransformConfiguration {
    /// Creates OTTL transform settings.
    pub fn new(error_mode: OttlErrorMode, trace_statements: Vec<String>) -> Self {
        Self {
            error_mode,
            trace_statements,
        }
    }

    /// Returns the configured error mode.
    pub const fn error_mode(&self) -> OttlErrorMode {
        self.error_mode
    }

    /// Returns configured trace transform statements.
    pub fn trace_statements(&self) -> &[String] {
        &self.trace_statements
    }
}

/// Native enablement for a simple component pipeline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PipelineConfiguration {
    enabled: bool,
}

impl PipelineConfiguration {
    /// Creates pipeline settings.
    pub const fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Returns whether the pipeline is enabled.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
}

/// Native OTLP pipeline settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OtlpPipelineConfiguration {
    enabled: bool,
    proxy: OtlpProxyConfiguration,
}

impl OtlpPipelineConfiguration {
    /// Creates OTLP pipeline settings.
    pub const fn new(enabled: bool, proxy: OtlpProxyConfiguration) -> Self {
        Self { enabled, proxy }
    }

    /// Returns whether OTLP ingest is enabled.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Returns OTLP proxy settings.
    pub const fn proxy(&self) -> &OtlpProxyConfiguration {
        &self.proxy
    }
}

/// Native OTLP proxy settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OtlpProxyConfiguration {
    enabled: bool,
    core_agent_otlp_grpc_endpoint: String,
    proxy_metrics: bool,
    proxy_logs: bool,
    proxy_traces: bool,
}

impl OtlpProxyConfiguration {
    /// Creates OTLP proxy settings.
    pub fn new(
        enabled: bool, core_agent_otlp_grpc_endpoint: String, proxy_metrics: bool, proxy_logs: bool, proxy_traces: bool,
    ) -> Self {
        Self {
            enabled,
            core_agent_otlp_grpc_endpoint,
            proxy_metrics,
            proxy_logs,
            proxy_traces,
        }
    }

    /// Returns whether OTLP proxy mode is enabled.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the Core Agent OTLP gRPC endpoint.
    pub fn core_agent_otlp_grpc_endpoint(&self) -> &str {
        &self.core_agent_otlp_grpc_endpoint
    }

    /// Returns whether metrics are proxied.
    pub const fn proxy_metrics(&self) -> bool {
        self.proxy_metrics
    }

    /// Returns whether logs are proxied.
    pub const fn proxy_logs(&self) -> bool {
        self.proxy_logs
    }

    /// Returns whether traces are proxied.
    pub const fn proxy_traces(&self) -> bool {
        self.proxy_traces
    }
}

/// Native OTLP receiver settings.
#[derive(Clone, Debug)]
pub struct OtlpReceiverConfiguration {
    http_endpoint: ListenAddress,
    grpc_endpoint: ListenAddress,
    grpc_max_recv_msg_size_bytes: usize,
}

impl OtlpReceiverConfiguration {
    /// Creates OTLP receiver settings.
    pub const fn new(
        http_endpoint: ListenAddress, grpc_endpoint: ListenAddress, grpc_max_recv_msg_size_bytes: usize,
    ) -> Self {
        Self {
            http_endpoint,
            grpc_endpoint,
            grpc_max_recv_msg_size_bytes,
        }
    }

    /// Returns the HTTP listen endpoint.
    pub const fn http_endpoint(&self) -> &ListenAddress {
        &self.http_endpoint
    }

    /// Returns the gRPC listen endpoint.
    pub const fn grpc_endpoint(&self) -> &ListenAddress {
        &self.grpc_endpoint
    }

    /// Returns the maximum accepted gRPC message size in bytes.
    pub const fn grpc_max_recv_msg_size_bytes(&self) -> usize {
        self.grpc_max_recv_msg_size_bytes
    }
}

/// Native OTLP trace processing settings.
#[derive(Clone, Debug, PartialEq)]
pub struct OtlpTracesConfiguration {
    enabled: bool,
    ignore_missing_datadog_fields: bool,
    enable_otlp_compute_top_level_by_span_kind: bool,
    probabilistic_sampler_sampling_percentage: f64,
    string_interner_bytes: usize,
    internal_port: u16,
}

impl OtlpTracesConfiguration {
    /// Creates OTLP trace processing settings.
    pub const fn new(
        enabled: bool, ignore_missing_datadog_fields: bool, enable_otlp_compute_top_level_by_span_kind: bool,
        probabilistic_sampler_sampling_percentage: f64, string_interner_bytes: usize, internal_port: u16,
    ) -> Self {
        Self {
            enabled,
            ignore_missing_datadog_fields,
            enable_otlp_compute_top_level_by_span_kind,
            probabilistic_sampler_sampling_percentage,
            string_interner_bytes,
            internal_port,
        }
    }

    /// Returns whether OTLP traces should be processed.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Returns whether Datadog-specific field fallbacks should be skipped.
    pub const fn ignore_missing_datadog_fields(&self) -> bool {
        self.ignore_missing_datadog_fields
    }

    /// Returns whether top-level span metadata should be computed from span kind.
    pub const fn enable_otlp_compute_top_level_by_span_kind(&self) -> bool {
        self.enable_otlp_compute_top_level_by_span_kind
    }

    /// Returns the OTLP probabilistic sampler percentage.
    pub const fn probabilistic_sampler_sampling_percentage(&self) -> f64 {
        self.probabilistic_sampler_sampling_percentage
    }

    /// Returns the trace string interner size in bytes.
    pub const fn string_interner_bytes(&self) -> usize {
        self.string_interner_bytes
    }

    /// Returns the internal Core Agent trace port.
    pub const fn internal_port(&self) -> u16 {
        self.internal_port
    }
}

/// Native OTLP source settings.
#[derive(Clone, Debug)]
pub struct OtlpSourceConfiguration {
    receiver: OtlpReceiverConfiguration,
    metrics_enabled: bool,
    logs_enabled: bool,
    traces: OtlpTracesConfiguration,
    context_string_interner_bytes: usize,
    cached_contexts_limit: usize,
    cached_tagsets_limit: usize,
    allow_context_heap_allocations: bool,
}

impl OtlpSourceConfiguration {
    /// Creates OTLP source settings.
    pub const fn new(
        receiver: OtlpReceiverConfiguration, metrics_enabled: bool, logs_enabled: bool,
        traces: OtlpTracesConfiguration, context_string_interner_bytes: usize, cached_contexts_limit: usize,
        cached_tagsets_limit: usize, allow_context_heap_allocations: bool,
    ) -> Self {
        Self {
            receiver,
            metrics_enabled,
            logs_enabled,
            traces,
            context_string_interner_bytes,
            cached_contexts_limit,
            cached_tagsets_limit,
            allow_context_heap_allocations,
        }
    }

    /// Returns the OTLP receiver settings.
    pub const fn receiver(&self) -> &OtlpReceiverConfiguration {
        &self.receiver
    }

    /// Returns whether OTLP metrics should be processed.
    pub const fn metrics_enabled(&self) -> bool {
        self.metrics_enabled
    }

    /// Returns whether OTLP logs should be processed.
    pub const fn logs_enabled(&self) -> bool {
        self.logs_enabled
    }

    /// Returns the OTLP traces settings.
    pub const fn traces(&self) -> &OtlpTracesConfiguration {
        &self.traces
    }

    /// Returns the metric context string interner size in bytes.
    pub const fn context_string_interner_bytes(&self) -> usize {
        self.context_string_interner_bytes
    }

    /// Returns the cached-context limit.
    pub const fn cached_contexts_limit(&self) -> usize {
        self.cached_contexts_limit
    }

    /// Returns the cached-tagset limit.
    pub const fn cached_tagsets_limit(&self) -> usize {
        self.cached_tagsets_limit
    }

    /// Returns whether context resolution may allocate on the heap when interners are full.
    pub const fn allow_context_heap_allocations(&self) -> bool {
        self.allow_context_heap_allocations
    }
}

/// Native OTLP forwarder settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OtlpForwarderConfiguration {
    core_agent_otlp_grpc_endpoint: String,
    core_agent_traces_internal_port: u16,
}

impl OtlpForwarderConfiguration {
    /// Creates OTLP forwarder settings.
    pub fn new(core_agent_otlp_grpc_endpoint: String, core_agent_traces_internal_port: u16) -> Self {
        Self {
            core_agent_otlp_grpc_endpoint,
            core_agent_traces_internal_port,
        }
    }

    /// Returns the Core Agent OTLP gRPC endpoint.
    pub fn core_agent_otlp_grpc_endpoint(&self) -> &str {
        &self.core_agent_otlp_grpc_endpoint
    }

    /// Returns the Trace Agent internal OTLP port.
    pub const fn core_agent_traces_internal_port(&self) -> u16 {
        self.core_agent_traces_internal_port
    }
}

/// Native Checks IPC source settings.
#[derive(Clone, Debug)]
pub struct ChecksIpcConfiguration {
    grpc_endpoint: ListenAddress,
}

impl ChecksIpcConfiguration {
    /// Creates Checks IPC settings.
    pub const fn new(grpc_endpoint: ListenAddress) -> Self {
        Self { grpc_endpoint }
    }

    /// Returns the gRPC listen endpoint.
    pub const fn grpc_endpoint(&self) -> &ListenAddress {
        &self.grpc_endpoint
    }
}

/// Native Datadog logs encoder settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatadogLogsEncoderConfiguration {
    compressor_kind: String,
    zstd_compressor_level: i32,
}

impl DatadogLogsEncoderConfiguration {
    /// Creates Datadog logs encoder settings.
    pub fn new(compressor_kind: String, zstd_compressor_level: i32) -> Self {
        Self {
            compressor_kind,
            zstd_compressor_level,
        }
    }

    /// Returns the compression algorithm name.
    pub fn compressor_kind(&self) -> &str {
        &self.compressor_kind
    }

    /// Returns the zstd compression level.
    pub const fn zstd_compressor_level(&self) -> i32 {
        self.zstd_compressor_level
    }
}

/// Native Datadog events encoder settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatadogEventsEncoderConfiguration {
    max_payload_size: usize,
    max_uncompressed_payload_size: usize,
    compressor_kind: String,
    zstd_compressor_level: i32,
    log_payloads: bool,
}

impl DatadogEventsEncoderConfiguration {
    /// Creates Datadog events encoder settings.
    pub fn new(
        max_payload_size: usize, max_uncompressed_payload_size: usize, compressor_kind: String,
        zstd_compressor_level: i32, log_payloads: bool,
    ) -> Self {
        Self {
            max_payload_size,
            max_uncompressed_payload_size,
            compressor_kind,
            zstd_compressor_level,
            log_payloads,
        }
    }

    /// Returns the maximum compressed payload size.
    pub const fn max_payload_size(&self) -> usize {
        self.max_payload_size
    }

    /// Returns the maximum uncompressed payload size.
    pub const fn max_uncompressed_payload_size(&self) -> usize {
        self.max_uncompressed_payload_size
    }

    /// Returns the compression algorithm name.
    pub fn compressor_kind(&self) -> &str {
        &self.compressor_kind
    }

    /// Returns the zstd compression level.
    pub const fn zstd_compressor_level(&self) -> i32 {
        self.zstd_compressor_level
    }

    /// Returns whether decoded payloads should be logged.
    pub const fn log_payloads(&self) -> bool {
        self.log_payloads
    }
}

/// Native Datadog service-checks encoder settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatadogServiceChecksEncoderConfiguration {
    max_payload_size: usize,
    max_uncompressed_payload_size: usize,
    compressor_kind: String,
    zstd_compressor_level: i32,
    log_payloads: bool,
}

impl DatadogServiceChecksEncoderConfiguration {
    /// Creates Datadog service-checks encoder settings.
    pub fn new(
        max_payload_size: usize, max_uncompressed_payload_size: usize, compressor_kind: String,
        zstd_compressor_level: i32, log_payloads: bool,
    ) -> Self {
        Self {
            max_payload_size,
            max_uncompressed_payload_size,
            compressor_kind,
            zstd_compressor_level,
            log_payloads,
        }
    }

    /// Returns the maximum compressed payload size.
    pub const fn max_payload_size(&self) -> usize {
        self.max_payload_size
    }

    /// Returns the maximum uncompressed payload size.
    pub const fn max_uncompressed_payload_size(&self) -> usize {
        self.max_uncompressed_payload_size
    }

    /// Returns the compression algorithm name.
    pub fn compressor_kind(&self) -> &str {
        &self.compressor_kind
    }

    /// Returns the zstd compression level.
    pub const fn zstd_compressor_level(&self) -> i32 {
        self.zstd_compressor_level
    }

    /// Returns whether decoded payloads should be logged.
    pub const fn log_payloads(&self) -> bool {
        self.log_payloads
    }
}

/// Native DogStatsD prefix/filter transform settings.
#[derive(Clone, Debug)]
pub struct DogStatsDPrefixFilterConfiguration {
    metric_prefix: String,
    metric_prefix_blocklist: Vec<String>,
    metric_filterlist: DynamicValue<Vec<String>>,
    metric_filterlist_match_prefix: DynamicValue<bool>,
    metric_blocklist: DynamicValue<Vec<String>>,
    metric_blocklist_match_prefix: DynamicValue<bool>,
}

impl DogStatsDPrefixFilterConfiguration {
    /// Creates DogStatsD prefix/filter transform settings.
    pub fn new(
        metric_prefix: String, metric_prefix_blocklist: Vec<String>, metric_filterlist: DynamicValue<Vec<String>>,
        metric_filterlist_match_prefix: DynamicValue<bool>, metric_blocklist: DynamicValue<Vec<String>>,
        metric_blocklist_match_prefix: DynamicValue<bool>,
    ) -> Self {
        Self {
            metric_prefix,
            metric_prefix_blocklist,
            metric_filterlist,
            metric_filterlist_match_prefix,
            metric_blocklist,
            metric_blocklist_match_prefix,
        }
    }

    /// Returns the metric prefix.
    pub fn metric_prefix(&self) -> &str {
        &self.metric_prefix
    }

    /// Returns prefixes excluded from prefixing.
    pub fn metric_prefix_blocklist(&self) -> &[String] {
        &self.metric_prefix_blocklist
    }

    /// Returns dynamic metric filterlist settings.
    pub fn metric_filterlist(&self) -> DynamicValue<Vec<String>> {
        self.metric_filterlist.clone()
    }

    /// Returns dynamic metric filterlist match-prefix settings.
    pub fn metric_filterlist_match_prefix(&self) -> DynamicValue<bool> {
        self.metric_filterlist_match_prefix.clone()
    }

    /// Returns dynamic metric blocklist settings.
    pub fn metric_blocklist(&self) -> DynamicValue<Vec<String>> {
        self.metric_blocklist.clone()
    }

    /// Returns dynamic metric blocklist match-prefix settings.
    pub fn metric_blocklist_match_prefix(&self) -> DynamicValue<bool> {
        self.metric_blocklist_match_prefix.clone()
    }
}

/// Native DogStatsD post-aggregate filter settings.
#[derive(Clone, Debug)]
pub struct DogStatsDPostAggregateFilterConfiguration {
    metric_filterlist: DynamicValue<Vec<String>>,
    metric_filterlist_match_prefix: DynamicValue<bool>,
    metric_blocklist: DynamicValue<Vec<String>>,
    metric_blocklist_match_prefix: DynamicValue<bool>,
    histogram_aggregates: Vec<String>,
    histogram_percentiles: Vec<String>,
}

impl DogStatsDPostAggregateFilterConfiguration {
    /// Creates DogStatsD post-aggregate filter settings.
    pub fn new(
        metric_filterlist: DynamicValue<Vec<String>>, metric_filterlist_match_prefix: DynamicValue<bool>,
        metric_blocklist: DynamicValue<Vec<String>>, metric_blocklist_match_prefix: DynamicValue<bool>,
        histogram_aggregates: Vec<String>, histogram_percentiles: Vec<String>,
    ) -> Self {
        Self {
            metric_filterlist,
            metric_filterlist_match_prefix,
            metric_blocklist,
            metric_blocklist_match_prefix,
            histogram_aggregates,
            histogram_percentiles,
        }
    }

    /// Returns dynamic metric filterlist settings.
    pub fn metric_filterlist(&self) -> DynamicValue<Vec<String>> {
        self.metric_filterlist.clone()
    }

    /// Returns dynamic metric filterlist match-prefix settings.
    pub fn metric_filterlist_match_prefix(&self) -> DynamicValue<bool> {
        self.metric_filterlist_match_prefix.clone()
    }

    /// Returns dynamic metric blocklist settings.
    pub fn metric_blocklist(&self) -> DynamicValue<Vec<String>> {
        self.metric_blocklist.clone()
    }

    /// Returns dynamic metric blocklist match-prefix settings.
    pub fn metric_blocklist_match_prefix(&self) -> DynamicValue<bool> {
        self.metric_blocklist_match_prefix.clone()
    }

    /// Returns histogram aggregate suffixes.
    pub fn histogram_aggregates(&self) -> &[String] {
        &self.histogram_aggregates
    }

    /// Returns histogram percentile suffixes.
    pub fn histogram_percentiles(&self) -> &[String] {
        &self.histogram_percentiles
    }
}

/// Action applied to a configured metric tag filterlist.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MetricTagFilterAction {
    /// Keep only listed tag keys.
    Include,
    /// Remove listed tag keys.
    #[default]
    Exclude,
}

/// Native metric tag filter entry.
#[derive(Clone, Debug)]
pub struct MetricTagFilterEntry {
    metric_name: String,
    action: MetricTagFilterAction,
    tags: Vec<String>,
}

impl MetricTagFilterEntry {
    /// Creates a metric tag filter entry.
    pub fn new(metric_name: String, action: MetricTagFilterAction, tags: Vec<String>) -> Self {
        Self {
            metric_name,
            action,
            tags,
        }
    }

    /// Returns the metric name.
    pub fn metric_name(&self) -> &str {
        &self.metric_name
    }

    /// Returns the filter action.
    pub const fn action(&self) -> MetricTagFilterAction {
        self.action
    }

    /// Returns tag keys targeted by this entry.
    pub fn tags(&self) -> &[String] {
        &self.tags
    }
}

/// Native metric tag filterlist settings.
#[derive(Clone, Debug)]
pub struct TagFilterlistConfiguration {
    entries: DynamicValue<Vec<MetricTagFilterEntry>>,
    context_cache_capacity: usize,
}

impl TagFilterlistConfiguration {
    /// Creates metric tag filterlist settings.
    pub fn new(entries: DynamicValue<Vec<MetricTagFilterEntry>>, context_cache_capacity: usize) -> Self {
        Self {
            entries,
            context_cache_capacity,
        }
    }

    /// Returns dynamic metric tag filter entries.
    pub fn entries(&self) -> DynamicValue<Vec<MetricTagFilterEntry>> {
        self.entries.clone()
    }

    /// Returns context cache capacity.
    pub const fn context_cache_capacity(&self) -> usize {
        self.context_cache_capacity
    }
}

/// Native DogStatsD aggregation transform settings.
#[derive(Clone, Debug)]
pub struct AggregateConfiguration {
    window_duration_seconds: NonZeroU64,
    primary_flush_interval: Duration,
    context_limit: usize,
    flush_open_windows: bool,
    counter_expiry_seconds: Option<u64>,
    passthrough_timestamped_metrics: bool,
    passthrough_idle_flush_timeout: Duration,
    histogram_aggregates: Vec<String>,
    histogram_percentiles: Vec<String>,
    histogram_copy_to_distribution: bool,
    histogram_copy_to_distribution_prefix: String,
}

impl AggregateConfiguration {
    /// Creates native DogStatsD aggregation transform settings.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        window_duration_seconds: NonZeroU64, primary_flush_interval: Duration, context_limit: usize,
        flush_open_windows: bool, counter_expiry_seconds: Option<u64>, passthrough_timestamped_metrics: bool,
        passthrough_idle_flush_timeout: Duration, histogram_aggregates: Vec<String>,
        histogram_percentiles: Vec<String>, histogram_copy_to_distribution: bool,
        histogram_copy_to_distribution_prefix: String,
    ) -> Self {
        Self {
            window_duration_seconds,
            primary_flush_interval,
            context_limit,
            flush_open_windows,
            counter_expiry_seconds,
            passthrough_timestamped_metrics,
            passthrough_idle_flush_timeout,
            histogram_aggregates,
            histogram_percentiles,
            histogram_copy_to_distribution,
            histogram_copy_to_distribution_prefix,
        }
    }

    /// Returns the aggregation window size in seconds.
    pub const fn window_duration_seconds(&self) -> NonZeroU64 {
        self.window_duration_seconds
    }

    /// Returns the primary flush interval.
    pub const fn primary_flush_interval(&self) -> Duration {
        self.primary_flush_interval
    }

    /// Returns the maximum number of contexts per window.
    pub const fn context_limit(&self) -> usize {
        self.context_limit
    }

    /// Returns whether open windows should be flushed on shutdown.
    pub const fn flush_open_windows(&self) -> bool {
        self.flush_open_windows
    }

    /// Returns idle counter expiration in seconds.
    pub const fn counter_expiry_seconds(&self) -> Option<u64> {
        self.counter_expiry_seconds
    }

    /// Returns whether timestamped metrics bypass aggregation.
    pub const fn passthrough_timestamped_metrics(&self) -> bool {
        self.passthrough_timestamped_metrics
    }

    /// Returns how long passthrough metrics may sit idle before flushing.
    pub const fn passthrough_idle_flush_timeout(&self) -> Duration {
        self.passthrough_idle_flush_timeout
    }

    /// Returns histogram aggregate names.
    pub fn histogram_aggregates(&self) -> &[String] {
        &self.histogram_aggregates
    }

    /// Returns histogram percentile quantiles.
    pub fn histogram_percentiles(&self) -> &[String] {
        &self.histogram_percentiles
    }

    /// Returns whether histograms should be copied to distributions.
    pub const fn histogram_copy_to_distribution(&self) -> bool {
        self.histogram_copy_to_distribution
    }

    /// Returns the prefix used for distributions copied from histograms.
    pub fn histogram_copy_to_distribution_prefix(&self) -> &str {
        &self.histogram_copy_to_distribution_prefix
    }
}

/// Native DogStatsD mapper transform settings.
#[derive(Clone, Debug)]
pub struct DogStatsDMapperConfiguration {
    context_string_interner_bytes: usize,
    cache_size: usize,
    profiles: Vec<DogStatsDMapperProfileConfiguration>,
}

impl DogStatsDMapperConfiguration {
    /// Creates native DogStatsD mapper transform settings.
    pub fn new(
        context_string_interner_bytes: usize, cache_size: usize, profiles: Vec<DogStatsDMapperProfileConfiguration>,
    ) -> Self {
        Self {
            context_string_interner_bytes,
            cache_size,
            profiles,
        }
    }

    /// Returns the string interner capacity in bytes.
    pub const fn context_string_interner_bytes(&self) -> usize {
        self.context_string_interner_bytes
    }

    /// Returns the maximum number of mapper results to cache.
    pub const fn cache_size(&self) -> usize {
        self.cache_size
    }

    /// Returns configured mapper profiles.
    pub fn profiles(&self) -> &[DogStatsDMapperProfileConfiguration] {
        &self.profiles
    }
}

/// Native DogStatsD mapper profile settings.
#[derive(Clone, Debug)]
pub struct DogStatsDMapperProfileConfiguration {
    name: String,
    prefix: String,
    mappings: Vec<DogStatsDMetricMappingConfiguration>,
}

impl DogStatsDMapperProfileConfiguration {
    /// Creates native DogStatsD mapper profile settings.
    pub fn new(name: String, prefix: String, mappings: Vec<DogStatsDMetricMappingConfiguration>) -> Self {
        Self { name, prefix, mappings }
    }

    /// Returns the profile name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the metric-name prefix matched by this profile.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Returns profile mappings.
    pub fn mappings(&self) -> &[DogStatsDMetricMappingConfiguration] {
        &self.mappings
    }
}

/// Native DogStatsD metric mapping settings.
#[derive(Clone, Debug)]
pub struct DogStatsDMetricMappingConfiguration {
    metric_match: String,
    match_type: String,
    name: String,
    tags: HashMap<String, String>,
}

impl DogStatsDMetricMappingConfiguration {
    /// Creates native DogStatsD metric mapping settings.
    pub fn new(metric_match: String, match_type: String, name: String, tags: HashMap<String, String>) -> Self {
        Self {
            metric_match,
            match_type,
            name,
            tags,
        }
    }

    /// Returns the wildcard or regex match expression.
    pub fn metric_match(&self) -> &str {
        &self.metric_match
    }

    /// Returns the match type.
    pub fn match_type(&self) -> &str {
        &self.match_type
    }

    /// Returns the mapped metric name expression.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns mapped tags.
    pub fn tags(&self) -> &HashMap<String, String> {
        &self.tags
    }
}

const MRF_METRICS_ENDPOINT_PREFIX: &str = "https://app.mrf.";

/// Native multi-region failover settings.
#[derive(Clone, Debug)]
pub struct MultiRegionFailoverConfiguration {
    enabled: bool,
    failover_metrics: DynamicValue<bool>,
    metric_allowlist: DynamicValue<Vec<String>>,
    api_key: Option<String>,
    site: Option<String>,
    dd_url: Option<String>,
}

impl MultiRegionFailoverConfiguration {
    /// Creates native multi-region failover settings.
    pub fn new(
        enabled: bool, failover_metrics: DynamicValue<bool>, metric_allowlist: DynamicValue<Vec<String>>,
        api_key: Option<String>, site: Option<String>, dd_url: Option<String>,
    ) -> Self {
        Self {
            enabled,
            failover_metrics,
            metric_allowlist,
            api_key,
            site,
            dd_url,
        }
    }

    /// Returns whether multi-region failover is enabled.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the dynamic failover-metrics setting.
    pub fn failover_metrics(&self) -> DynamicValue<bool> {
        self.failover_metrics.clone()
    }

    /// Returns the dynamic metric allowlist setting.
    pub fn metric_allowlist(&self) -> DynamicValue<Vec<String>> {
        self.metric_allowlist.clone()
    }

    /// Returns the failover-region API key.
    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    /// Returns the configured failover site.
    pub fn site(&self) -> Option<&str> {
        self.site.as_deref()
    }

    /// Returns the explicit failover Datadog URL.
    pub fn dd_url(&self) -> Option<&str> {
        self.dd_url.as_deref()
    }

    /// Returns whether metrics forwarding to the failover region is requested by configuration.
    pub fn is_metrics_forwarding_requested(&self) -> bool {
        self.enabled && self.failover_metrics.current()
    }

    /// Returns the failover-region metrics endpoint URL.
    pub fn metrics_endpoint_url(&self) -> Option<String> {
        self.dd_url.clone().or_else(|| {
            self.site
                .as_deref()
                .map(|site| format!("{MRF_METRICS_ENDPOINT_PREFIX}{site}"))
        })
    }

    /// Returns the endpoint and API key override for the failover-region metrics forwarder.
    pub fn metrics_endpoint_override(&self) -> Option<(String, String)> {
        if !self.enabled {
            return None;
        }

        Some((self.metrics_endpoint_url()?, self.api_key.clone()?))
    }
}

/// Native Datadog forwarder HTTP protocol selection.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DatadogForwarderHttpProtocol {
    /// Automatically negotiate HTTP/2 with HTTP/1.1 fallback.
    #[default]
    Auto,
    /// Use HTTP/1.1 only.
    Http1,
}

/// Native Datadog forwarder endpoint settings.
#[derive(Clone, Debug)]
pub struct DatadogForwarderEndpointConfiguration {
    api_key: DynamicValue<String>,
    site: String,
    dd_url: Option<String>,
    additional_endpoints: DynamicValue<HashMap<String, Vec<String>>>,
}

impl DatadogForwarderEndpointConfiguration {
    /// Creates native Datadog endpoint settings.
    pub fn new(
        api_key: DynamicValue<String>, site: String, dd_url: Option<String>,
        additional_endpoints: DynamicValue<HashMap<String, Vec<String>>>,
    ) -> Self {
        Self {
            api_key,
            site,
            dd_url,
            additional_endpoints,
        }
    }

    /// Returns the dynamic primary API key.
    pub fn api_key(&self) -> DynamicValue<String> {
        self.api_key.clone()
    }

    /// Returns the configured Datadog site.
    pub fn site(&self) -> &str {
        &self.site
    }

    /// Returns the configured Datadog URL override.
    pub fn dd_url(&self) -> Option<&str> {
        self.dd_url.as_deref()
    }

    /// Returns the dynamic additional endpoint map.
    pub fn additional_endpoints(&self) -> DynamicValue<HashMap<String, Vec<String>>> {
        self.additional_endpoints.clone()
    }

    /// Returns a clone overriding the primary endpoint and API key and clearing additional endpoints.
    pub fn with_primary_endpoint_override(&self, dd_url: String, api_key: DynamicValue<String>) -> Self {
        Self {
            api_key,
            site: self.site.clone(),
            dd_url: Some(dd_url),
            additional_endpoints: DynamicValue::fixed(HashMap::new()),
        }
    }
}

/// Native Datadog forwarder retry settings.
#[derive(Clone, Debug)]
pub struct DatadogForwarderRetryConfiguration {
    backoff_factor: f64,
    backoff_base_secs: f64,
    backoff_max_secs: f64,
    recovery_error_decrease_factor: u32,
    recovery_reset: bool,
    retry_queue_payloads_max_size_bytes: u64,
    storage_max_size_bytes: u64,
    storage_path: PathBuf,
    storage_max_disk_ratio: f64,
    outdated_file_in_days: u32,
    capacity_time_interval_secs: u64,
    retry_forbidden_when_secrets_in_use: DynamicValue<bool>,
}

impl DatadogForwarderRetryConfiguration {
    /// Creates native Datadog forwarder retry settings.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        backoff_factor: f64, backoff_base_secs: f64, backoff_max_secs: f64, recovery_error_decrease_factor: u32,
        recovery_reset: bool, retry_queue_payloads_max_size_bytes: u64, storage_max_size_bytes: u64,
        storage_path: PathBuf, storage_max_disk_ratio: f64, outdated_file_in_days: u32,
        capacity_time_interval_secs: u64, retry_forbidden_when_secrets_in_use: DynamicValue<bool>,
    ) -> Self {
        Self {
            backoff_factor,
            backoff_base_secs,
            backoff_max_secs,
            recovery_error_decrease_factor,
            recovery_reset,
            retry_queue_payloads_max_size_bytes,
            storage_max_size_bytes,
            storage_path,
            storage_max_disk_ratio,
            outdated_file_in_days,
            capacity_time_interval_secs,
            retry_forbidden_when_secrets_in_use,
        }
    }

    pub const fn backoff_factor(&self) -> f64 {
        self.backoff_factor
    }
    pub const fn backoff_base_secs(&self) -> f64 {
        self.backoff_base_secs
    }
    pub const fn backoff_max_secs(&self) -> f64 {
        self.backoff_max_secs
    }
    pub const fn recovery_error_decrease_factor(&self) -> u32 {
        self.recovery_error_decrease_factor
    }
    pub const fn recovery_reset(&self) -> bool {
        self.recovery_reset
    }
    pub const fn retry_queue_payloads_max_size_bytes(&self) -> u64 {
        self.retry_queue_payloads_max_size_bytes
    }
    pub const fn storage_max_size_bytes(&self) -> u64 {
        self.storage_max_size_bytes
    }
    pub fn storage_path(&self) -> &PathBuf {
        &self.storage_path
    }
    pub const fn storage_max_disk_ratio(&self) -> f64 {
        self.storage_max_disk_ratio
    }
    pub const fn outdated_file_in_days(&self) -> u32 {
        self.outdated_file_in_days
    }
    pub const fn capacity_time_interval_secs(&self) -> u64 {
        self.capacity_time_interval_secs
    }
    pub fn retry_forbidden_when_secrets_in_use(&self) -> DynamicValue<bool> {
        self.retry_forbidden_when_secrets_in_use.clone()
    }
}

/// Native Datadog proxy settings.
#[derive(Clone, Debug, Default)]
pub struct DatadogProxyConfiguration {
    http_server: Option<String>,
    https_server: Option<String>,
    no_proxy: Vec<String>,
    no_proxy_nonexact_match: bool,
    use_proxy_for_cloud_metadata: bool,
}

impl DatadogProxyConfiguration {
    /// Creates native Datadog proxy settings.
    pub fn new(
        http_server: Option<String>, https_server: Option<String>, no_proxy: Vec<String>,
        no_proxy_nonexact_match: bool, use_proxy_for_cloud_metadata: bool,
    ) -> Self {
        Self {
            http_server,
            https_server,
            no_proxy,
            no_proxy_nonexact_match,
            use_proxy_for_cloud_metadata,
        }
    }

    pub fn http_server(&self) -> Option<&str> {
        self.http_server.as_deref()
    }
    pub fn https_server(&self) -> Option<&str> {
        self.https_server.as_deref()
    }
    pub fn no_proxy(&self) -> &[String] {
        &self.no_proxy
    }
    pub const fn no_proxy_nonexact_match(&self) -> bool {
        self.no_proxy_nonexact_match
    }
    pub const fn use_proxy_for_cloud_metadata(&self) -> bool {
        self.use_proxy_for_cloud_metadata
    }
}

/// Native OPW/Vector metrics endpoint override settings.
#[derive(Clone, Debug, Default)]
pub struct DatadogOpwMetricsConfiguration {
    observability_pipelines_worker_enabled: bool,
    observability_pipelines_worker_url: String,
    vector_enabled: bool,
    vector_url: String,
}

impl DatadogOpwMetricsConfiguration {
    /// Creates native OPW/Vector metrics endpoint override settings.
    pub fn new(
        observability_pipelines_worker_enabled: bool, observability_pipelines_worker_url: String, vector_enabled: bool,
        vector_url: String,
    ) -> Self {
        Self {
            observability_pipelines_worker_enabled,
            observability_pipelines_worker_url,
            vector_enabled,
            vector_url,
        }
    }

    pub const fn observability_pipelines_worker_enabled(&self) -> bool {
        self.observability_pipelines_worker_enabled
    }
    pub fn observability_pipelines_worker_url(&self) -> &str {
        &self.observability_pipelines_worker_url
    }
    pub const fn vector_enabled(&self) -> bool {
        self.vector_enabled
    }
    pub fn vector_url(&self) -> &str {
        &self.vector_url
    }
}

/// Native Datadog forwarder settings.
#[derive(Clone, Debug)]
pub struct DatadogForwarderConfiguration {
    endpoint_concurrency: usize,
    endpoint_concurrency_multiplier: usize,
    request_timeout_secs: u64,
    endpoint_buffer_size: usize,
    endpoint: DatadogForwarderEndpointConfiguration,
    retry: DatadogForwarderRetryConfiguration,
    proxy: Option<DatadogProxyConfiguration>,
    opw_metrics: DatadogOpwMetricsConfiguration,
    http_protocol: DatadogForwarderHttpProtocol,
    connection_reset_interval_secs: u64,
    skip_ssl_validation: bool,
    ssl_key_log_file_path: Option<String>,
    min_tls_version: String,
    allow_arbitrary_tags: bool,
    api_key_validation_interval_mins: u64,
}

impl DatadogForwarderConfiguration {
    /// Creates native Datadog forwarder settings.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        endpoint_concurrency: usize, endpoint_concurrency_multiplier: usize, request_timeout_secs: u64,
        endpoint_buffer_size: usize, endpoint: DatadogForwarderEndpointConfiguration,
        retry: DatadogForwarderRetryConfiguration, proxy: Option<DatadogProxyConfiguration>,
        opw_metrics: DatadogOpwMetricsConfiguration, http_protocol: DatadogForwarderHttpProtocol,
        connection_reset_interval_secs: u64, skip_ssl_validation: bool, ssl_key_log_file_path: Option<String>,
        min_tls_version: String, allow_arbitrary_tags: bool, api_key_validation_interval_mins: u64,
    ) -> Self {
        Self {
            endpoint_concurrency,
            endpoint_concurrency_multiplier,
            request_timeout_secs,
            endpoint_buffer_size,
            endpoint,
            retry,
            proxy,
            opw_metrics,
            http_protocol,
            connection_reset_interval_secs,
            skip_ssl_validation,
            ssl_key_log_file_path,
            min_tls_version,
            allow_arbitrary_tags,
            api_key_validation_interval_mins,
        }
    }

    pub const fn endpoint_concurrency(&self) -> usize {
        self.endpoint_concurrency
    }
    pub const fn endpoint_concurrency_multiplier(&self) -> usize {
        self.endpoint_concurrency_multiplier
    }
    pub const fn request_timeout_secs(&self) -> u64 {
        self.request_timeout_secs
    }
    pub const fn endpoint_buffer_size(&self) -> usize {
        self.endpoint_buffer_size
    }
    pub const fn endpoint(&self) -> &DatadogForwarderEndpointConfiguration {
        &self.endpoint
    }
    pub const fn retry(&self) -> &DatadogForwarderRetryConfiguration {
        &self.retry
    }
    pub const fn proxy(&self) -> Option<&DatadogProxyConfiguration> {
        self.proxy.as_ref()
    }
    pub const fn opw_metrics(&self) -> &DatadogOpwMetricsConfiguration {
        &self.opw_metrics
    }
    pub const fn http_protocol(&self) -> DatadogForwarderHttpProtocol {
        self.http_protocol
    }
    pub const fn connection_reset_interval_secs(&self) -> u64 {
        self.connection_reset_interval_secs
    }
    pub const fn skip_ssl_validation(&self) -> bool {
        self.skip_ssl_validation
    }
    pub fn ssl_key_log_file_path(&self) -> Option<&str> {
        self.ssl_key_log_file_path.as_deref()
    }
    pub fn min_tls_version(&self) -> &str {
        &self.min_tls_version
    }
    pub const fn allow_arbitrary_tags(&self) -> bool {
        self.allow_arbitrary_tags
    }
    pub const fn api_key_validation_interval_mins(&self) -> u64 {
        self.api_key_validation_interval_mins
    }

    /// Returns a clone with a primary endpoint override and no OPW/additional endpoint routing.
    pub fn with_primary_endpoint_override(&self, dd_url: String, api_key: DynamicValue<String>) -> Self {
        let mut clone = self.clone();
        clone.endpoint = self.endpoint.with_primary_endpoint_override(dd_url, api_key);
        clone.opw_metrics = DatadogOpwMetricsConfiguration::default();
        clone
    }
}

/// Native trace obfuscation settings.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TraceObfuscationConfiguration {
    credit_cards_enabled: bool,
    credit_cards_luhn: bool,
    credit_cards_keep_values: Vec<String>,
    http_remove_query_string: bool,
    http_remove_paths_with_digits: bool,
    memcached_enabled: bool,
    memcached_keep_command: bool,
    redis_enabled: bool,
    redis_remove_all_args: bool,
    valkey_enabled: bool,
    valkey_remove_all_args: bool,
    sql_dbms: String,
    sql_table_names: bool,
    sql_replace_digits: bool,
    sql_keep_sql_alias: bool,
    sql_dollar_quoted_func: bool,
    mongodb_enabled: bool,
    mongodb_keep_values: Vec<String>,
    mongodb_obfuscate_sql_values: Vec<String>,
    elasticsearch_enabled: bool,
    elasticsearch_keep_values: Vec<String>,
    elasticsearch_obfuscate_sql_values: Vec<String>,
    opensearch_enabled: bool,
    opensearch_keep_values: Vec<String>,
    opensearch_obfuscate_sql_values: Vec<String>,
}

impl TraceObfuscationConfiguration {
    /// Creates native trace obfuscation settings.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        credit_cards_enabled: bool, credit_cards_luhn: bool, credit_cards_keep_values: Vec<String>,
        http_remove_query_string: bool, http_remove_paths_with_digits: bool, memcached_enabled: bool,
        memcached_keep_command: bool, redis_enabled: bool, redis_remove_all_args: bool, valkey_enabled: bool,
        valkey_remove_all_args: bool, sql_dbms: String, sql_table_names: bool, sql_replace_digits: bool,
        sql_keep_sql_alias: bool, sql_dollar_quoted_func: bool, mongodb_enabled: bool,
        mongodb_keep_values: Vec<String>, mongodb_obfuscate_sql_values: Vec<String>, elasticsearch_enabled: bool,
        elasticsearch_keep_values: Vec<String>, elasticsearch_obfuscate_sql_values: Vec<String>,
        opensearch_enabled: bool, opensearch_keep_values: Vec<String>, opensearch_obfuscate_sql_values: Vec<String>,
    ) -> Self {
        Self {
            credit_cards_enabled,
            credit_cards_luhn,
            credit_cards_keep_values,
            http_remove_query_string,
            http_remove_paths_with_digits,
            memcached_enabled,
            memcached_keep_command,
            redis_enabled,
            redis_remove_all_args,
            valkey_enabled,
            valkey_remove_all_args,
            sql_dbms,
            sql_table_names,
            sql_replace_digits,
            sql_keep_sql_alias,
            sql_dollar_quoted_func,
            mongodb_enabled,
            mongodb_keep_values,
            mongodb_obfuscate_sql_values,
            elasticsearch_enabled,
            elasticsearch_keep_values,
            elasticsearch_obfuscate_sql_values,
            opensearch_enabled,
            opensearch_keep_values,
            opensearch_obfuscate_sql_values,
        }
    }

    pub const fn credit_cards_enabled(&self) -> bool {
        self.credit_cards_enabled
    }
    pub const fn credit_cards_luhn(&self) -> bool {
        self.credit_cards_luhn
    }
    pub fn credit_cards_keep_values(&self) -> &[String] {
        &self.credit_cards_keep_values
    }
    pub const fn http_remove_query_string(&self) -> bool {
        self.http_remove_query_string
    }
    pub const fn http_remove_paths_with_digits(&self) -> bool {
        self.http_remove_paths_with_digits
    }
    pub const fn memcached_enabled(&self) -> bool {
        self.memcached_enabled
    }
    pub const fn memcached_keep_command(&self) -> bool {
        self.memcached_keep_command
    }
    pub const fn redis_enabled(&self) -> bool {
        self.redis_enabled
    }
    pub const fn redis_remove_all_args(&self) -> bool {
        self.redis_remove_all_args
    }
    pub const fn valkey_enabled(&self) -> bool {
        self.valkey_enabled
    }
    pub const fn valkey_remove_all_args(&self) -> bool {
        self.valkey_remove_all_args
    }
    pub fn sql_dbms(&self) -> &str {
        &self.sql_dbms
    }
    pub const fn sql_table_names(&self) -> bool {
        self.sql_table_names
    }
    pub const fn sql_replace_digits(&self) -> bool {
        self.sql_replace_digits
    }
    pub const fn sql_keep_sql_alias(&self) -> bool {
        self.sql_keep_sql_alias
    }
    pub const fn sql_dollar_quoted_func(&self) -> bool {
        self.sql_dollar_quoted_func
    }
    pub const fn mongodb_enabled(&self) -> bool {
        self.mongodb_enabled
    }
    pub fn mongodb_keep_values(&self) -> &[String] {
        &self.mongodb_keep_values
    }
    pub fn mongodb_obfuscate_sql_values(&self) -> &[String] {
        &self.mongodb_obfuscate_sql_values
    }
    pub const fn elasticsearch_enabled(&self) -> bool {
        self.elasticsearch_enabled
    }
    pub fn elasticsearch_keep_values(&self) -> &[String] {
        &self.elasticsearch_keep_values
    }
    pub fn elasticsearch_obfuscate_sql_values(&self) -> &[String] {
        &self.elasticsearch_obfuscate_sql_values
    }
    pub const fn opensearch_enabled(&self) -> bool {
        self.opensearch_enabled
    }
    pub fn opensearch_keep_values(&self) -> &[String] {
        &self.opensearch_keep_values
    }
    pub fn opensearch_obfuscate_sql_values(&self) -> &[String] {
        &self.opensearch_obfuscate_sql_values
    }
}

/// Native DogStatsD origin tag cardinality.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DogStatsDOriginTagCardinality {
    /// No origin tags.
    None,
    /// Low-cardinality origin tags.
    #[default]
    Low,
    /// Orchestrator-cardinality origin tags.
    Orchestrator,
    /// High-cardinality origin tags.
    High,
}

/// Native DogStatsD payload enablement settings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DogStatsDEnablePayloadsConfiguration {
    series: bool,
    sketches: bool,
    events: bool,
    service_checks: bool,
}

impl DogStatsDEnablePayloadsConfiguration {
    /// Creates DogStatsD payload enablement settings.
    pub const fn new(series: bool, sketches: bool, events: bool, service_checks: bool) -> Self {
        Self {
            series,
            sketches,
            events,
            service_checks,
        }
    }

    pub const fn series(&self) -> bool {
        self.series
    }
    pub const fn sketches(&self) -> bool {
        self.sketches
    }
    pub const fn events(&self) -> bool {
        self.events
    }
    pub const fn service_checks(&self) -> bool {
        self.service_checks
    }
}

impl Default for DogStatsDEnablePayloadsConfiguration {
    fn default() -> Self {
        Self {
            series: true,
            sketches: true,
            events: true,
            service_checks: true,
        }
    }
}

/// Native DogStatsD origin enrichment settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DogStatsDOriginEnrichmentConfiguration {
    enabled: bool,
    entity_id_precedence: bool,
    tag_cardinality: DogStatsDOriginTagCardinality,
    origin_detection_unified: bool,
    origin_detection_optout: bool,
    origin_detection_client: bool,
}

impl DogStatsDOriginEnrichmentConfiguration {
    /// Creates DogStatsD origin enrichment settings.
    pub const fn new(
        enabled: bool, entity_id_precedence: bool, tag_cardinality: DogStatsDOriginTagCardinality,
        origin_detection_unified: bool, origin_detection_optout: bool, origin_detection_client: bool,
    ) -> Self {
        Self {
            enabled,
            entity_id_precedence,
            tag_cardinality,
            origin_detection_unified,
            origin_detection_optout,
            origin_detection_client,
        }
    }

    pub const fn enabled(&self) -> bool {
        self.enabled
    }
    pub const fn entity_id_precedence(&self) -> bool {
        self.entity_id_precedence
    }
    pub const fn tag_cardinality(&self) -> DogStatsDOriginTagCardinality {
        self.tag_cardinality
    }
    pub const fn origin_detection_unified(&self) -> bool {
        self.origin_detection_unified
    }
    pub const fn origin_detection_optout(&self) -> bool {
        self.origin_detection_optout
    }
    pub const fn origin_detection_client(&self) -> bool {
        self.origin_detection_client
    }
}

impl Default for DogStatsDOriginEnrichmentConfiguration {
    fn default() -> Self {
        Self {
            enabled: false,
            entity_id_precedence: false,
            tag_cardinality: DogStatsDOriginTagCardinality::Low,
            origin_detection_unified: false,
            origin_detection_optout: true,
            origin_detection_client: false,
        }
    }
}

/// Native DogStatsD source settings.
#[derive(Clone, Debug)]
pub struct DogStatsDSourceConfiguration {
    buffer_size: usize,
    buffer_count: usize,
    port: u16,
    socket_receive_buffer_size: usize,
    tcp_port: u16,
    statsd_forward_host: Option<String>,
    statsd_forward_port: u16,
    socket_path: Option<String>,
    socket_stream_path: Option<String>,
    stream_log_too_big: bool,
    eol_required: Vec<String>,
    bind_host: Option<String>,
    non_local_traffic: bool,
    autoscale_udp_listeners: bool,
    allow_context_heap_allocations: bool,
    no_aggregation_pipeline_support: bool,
    context_string_interner_entry_count: u64,
    context_string_interner_size_bytes: Option<u64>,
    cached_contexts_limit: usize,
    cached_tagsets_limit: usize,
    context_expiry_seconds: u64,
    permissive_decoding: bool,
    minimum_sample_rate: f64,
    enable_payloads: DogStatsDEnablePayloadsConfiguration,
    origin_enrichment: DogStatsDOriginEnrichmentConfiguration,
    additional_tags: Vec<String>,
    capture_path: PathBuf,
    capture_depth: usize,
    provider_kind: String,
}

impl DogStatsDSourceConfiguration {
    /// Creates native DogStatsD source settings.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        buffer_size: usize, buffer_count: usize, port: u16, socket_receive_buffer_size: usize, tcp_port: u16,
        statsd_forward_host: Option<String>, statsd_forward_port: u16, socket_path: Option<String>,
        socket_stream_path: Option<String>, stream_log_too_big: bool, eol_required: Vec<String>,
        bind_host: Option<String>, non_local_traffic: bool, autoscale_udp_listeners: bool,
        allow_context_heap_allocations: bool, no_aggregation_pipeline_support: bool,
        context_string_interner_entry_count: u64, context_string_interner_size_bytes: Option<u64>,
        cached_contexts_limit: usize, cached_tagsets_limit: usize, context_expiry_seconds: u64,
        permissive_decoding: bool, minimum_sample_rate: f64, enable_payloads: DogStatsDEnablePayloadsConfiguration,
        origin_enrichment: DogStatsDOriginEnrichmentConfiguration, additional_tags: Vec<String>, capture_path: PathBuf,
        capture_depth: usize, provider_kind: String,
    ) -> Self {
        Self {
            buffer_size,
            buffer_count,
            port,
            socket_receive_buffer_size,
            tcp_port,
            statsd_forward_host,
            statsd_forward_port,
            socket_path,
            socket_stream_path,
            stream_log_too_big,
            eol_required,
            bind_host,
            non_local_traffic,
            autoscale_udp_listeners,
            allow_context_heap_allocations,
            no_aggregation_pipeline_support,
            context_string_interner_entry_count,
            context_string_interner_size_bytes,
            cached_contexts_limit,
            cached_tagsets_limit,
            context_expiry_seconds,
            permissive_decoding,
            minimum_sample_rate,
            enable_payloads,
            origin_enrichment,
            additional_tags,
            capture_path,
            capture_depth,
            provider_kind,
        }
    }

    pub const fn buffer_size(&self) -> usize {
        self.buffer_size
    }
    pub const fn buffer_count(&self) -> usize {
        self.buffer_count
    }
    pub const fn port(&self) -> u16 {
        self.port
    }
    pub const fn socket_receive_buffer_size(&self) -> usize {
        self.socket_receive_buffer_size
    }
    pub const fn tcp_port(&self) -> u16 {
        self.tcp_port
    }
    pub fn statsd_forward_host(&self) -> Option<&str> {
        self.statsd_forward_host.as_deref()
    }
    pub const fn statsd_forward_port(&self) -> u16 {
        self.statsd_forward_port
    }
    pub fn socket_path(&self) -> Option<&str> {
        self.socket_path.as_deref()
    }
    pub fn socket_stream_path(&self) -> Option<&str> {
        self.socket_stream_path.as_deref()
    }
    pub const fn stream_log_too_big(&self) -> bool {
        self.stream_log_too_big
    }
    pub fn eol_required(&self) -> &[String] {
        &self.eol_required
    }
    pub fn bind_host(&self) -> Option<&str> {
        self.bind_host.as_deref()
    }
    pub const fn non_local_traffic(&self) -> bool {
        self.non_local_traffic
    }
    pub const fn autoscale_udp_listeners(&self) -> bool {
        self.autoscale_udp_listeners
    }
    pub const fn allow_context_heap_allocations(&self) -> bool {
        self.allow_context_heap_allocations
    }
    pub const fn no_aggregation_pipeline_support(&self) -> bool {
        self.no_aggregation_pipeline_support
    }
    pub const fn context_string_interner_entry_count(&self) -> u64 {
        self.context_string_interner_entry_count
    }
    pub const fn context_string_interner_size_bytes(&self) -> Option<u64> {
        self.context_string_interner_size_bytes
    }
    pub const fn cached_contexts_limit(&self) -> usize {
        self.cached_contexts_limit
    }
    pub const fn cached_tagsets_limit(&self) -> usize {
        self.cached_tagsets_limit
    }
    pub const fn context_expiry_seconds(&self) -> u64 {
        self.context_expiry_seconds
    }
    pub const fn permissive_decoding(&self) -> bool {
        self.permissive_decoding
    }
    pub const fn minimum_sample_rate(&self) -> f64 {
        self.minimum_sample_rate
    }
    pub const fn enable_payloads(&self) -> DogStatsDEnablePayloadsConfiguration {
        self.enable_payloads
    }
    pub const fn origin_enrichment(&self) -> &DogStatsDOriginEnrichmentConfiguration {
        &self.origin_enrichment
    }
    pub fn additional_tags(&self) -> &[String] {
        &self.additional_tags
    }
    pub fn capture_path(&self) -> &PathBuf {
        &self.capture_path
    }
    pub const fn capture_depth(&self) -> usize {
        self.capture_depth
    }
    pub fn provider_kind(&self) -> &str {
        &self.provider_kind
    }
}

/// Native Datadog trace encoder settings.
#[derive(Clone, Debug)]
pub struct DatadogTraceEncoderConfiguration {
    compressor_kind: String,
    zstd_compressor_level: i32,
    flush_timeout_secs: u64,
    env: String,
    target_traces_per_second: f64,
    errors_per_second: f64,
    error_tracking_standalone_enabled: bool,
    otlp_ignore_missing_datadog_fields: bool,
    otlp_sampling_percentage: f64,
}

impl DatadogTraceEncoderConfiguration {
    /// Creates native Datadog trace encoder settings.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        compressor_kind: String, zstd_compressor_level: i32, flush_timeout_secs: u64, env: String,
        target_traces_per_second: f64, errors_per_second: f64, error_tracking_standalone_enabled: bool,
        otlp_ignore_missing_datadog_fields: bool, otlp_sampling_percentage: f64,
    ) -> Self {
        Self {
            compressor_kind,
            zstd_compressor_level,
            flush_timeout_secs,
            env,
            target_traces_per_second,
            errors_per_second,
            error_tracking_standalone_enabled,
            otlp_ignore_missing_datadog_fields,
            otlp_sampling_percentage,
        }
    }

    /// Returns serializer compressor kind.
    pub fn compressor_kind(&self) -> &str {
        &self.compressor_kind
    }

    /// Returns zstd compressor level.
    pub const fn zstd_compressor_level(&self) -> i32 {
        self.zstd_compressor_level
    }

    /// Returns flush timeout in seconds.
    pub const fn flush_timeout_secs(&self) -> u64 {
        self.flush_timeout_secs
    }

    /// Returns the default trace environment.
    pub fn env(&self) -> &str {
        &self.env
    }

    /// Returns target traces per second.
    pub const fn target_traces_per_second(&self) -> f64 {
        self.target_traces_per_second
    }

    /// Returns error traces per second.
    pub const fn errors_per_second(&self) -> f64 {
        self.errors_per_second
    }

    /// Returns whether error tracking standalone mode is enabled.
    pub const fn error_tracking_standalone_enabled(&self) -> bool {
        self.error_tracking_standalone_enabled
    }

    /// Returns whether missing Datadog OTLP fields are ignored.
    pub const fn otlp_ignore_missing_datadog_fields(&self) -> bool {
        self.otlp_ignore_missing_datadog_fields
    }

    /// Returns OTLP sampling percentage.
    pub const fn otlp_sampling_percentage(&self) -> f64 {
        self.otlp_sampling_percentage
    }
}

/// Native APM stats transform settings.
#[derive(Clone, Debug)]
pub struct ApmStatsTransformConfiguration {
    compute_stats_by_span_kind: bool,
    peer_tags_aggregation: bool,
    peer_tags: Vec<String>,
    default_env: String,
    hostname: String,
}

impl ApmStatsTransformConfiguration {
    /// Creates native APM stats transform settings.
    pub fn new(
        compute_stats_by_span_kind: bool, peer_tags_aggregation: bool, peer_tags: Vec<String>, default_env: String,
        hostname: String,
    ) -> Self {
        Self {
            compute_stats_by_span_kind,
            peer_tags_aggregation,
            peer_tags,
            default_env,
            hostname,
        }
    }

    /// Returns whether stats computation by span kind is enabled.
    pub const fn compute_stats_by_span_kind(&self) -> bool {
        self.compute_stats_by_span_kind
    }

    /// Returns whether peer tag aggregation is enabled.
    pub const fn peer_tags_aggregation(&self) -> bool {
        self.peer_tags_aggregation
    }

    /// Returns supplementary peer tags.
    pub fn peer_tags(&self) -> &[String] {
        &self.peer_tags
    }

    /// Returns the default trace environment.
    pub fn default_env(&self) -> &str {
        &self.default_env
    }

    /// Returns the configured trace hostname.
    pub fn hostname(&self) -> &str {
        &self.hostname
    }
}

/// Native trace sampler settings.
#[derive(Clone, Debug)]
pub struct TraceSamplerConfiguration {
    target_traces_per_second: f64,
    errors_per_second: f64,
    probabilistic_sampler_enabled: bool,
    probabilistic_sampler_sampling_percentage: f64,
    error_sampling_enabled: bool,
    error_tracking_standalone_enabled: bool,
    rare_sampler_enabled: bool,
    rare_sampler_tps: f64,
    rare_sampler_cooldown_period_secs: f64,
    rare_sampler_cardinality: usize,
    default_env: String,
    otlp_sampling_rate: f64,
}

impl TraceSamplerConfiguration {
    /// Creates native trace sampler settings.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        target_traces_per_second: f64, errors_per_second: f64, probabilistic_sampler_enabled: bool,
        probabilistic_sampler_sampling_percentage: f64, error_sampling_enabled: bool,
        error_tracking_standalone_enabled: bool, rare_sampler_enabled: bool, rare_sampler_tps: f64,
        rare_sampler_cooldown_period_secs: f64, rare_sampler_cardinality: usize, default_env: String,
        otlp_sampling_rate: f64,
    ) -> Self {
        Self {
            target_traces_per_second,
            errors_per_second,
            probabilistic_sampler_enabled,
            probabilistic_sampler_sampling_percentage,
            error_sampling_enabled,
            error_tracking_standalone_enabled,
            rare_sampler_enabled,
            rare_sampler_tps,
            rare_sampler_cooldown_period_secs,
            rare_sampler_cardinality,
            default_env,
            otlp_sampling_rate,
        }
    }

    /// Returns the target traces per second for priority sampling.
    pub const fn target_traces_per_second(&self) -> f64 {
        self.target_traces_per_second
    }

    /// Returns the target traces per second for error sampling.
    pub const fn errors_per_second(&self) -> f64 {
        self.errors_per_second
    }

    /// Returns whether probabilistic sampling is enabled.
    pub const fn probabilistic_sampler_enabled(&self) -> bool {
        self.probabilistic_sampler_enabled
    }

    /// Returns the probabilistic sampler percentage.
    pub const fn probabilistic_sampler_sampling_percentage(&self) -> f64 {
        self.probabilistic_sampler_sampling_percentage
    }

    /// Returns whether error sampling is enabled.
    pub const fn error_sampling_enabled(&self) -> bool {
        self.error_sampling_enabled
    }

    /// Returns whether error tracking standalone mode is enabled.
    pub const fn error_tracking_standalone_enabled(&self) -> bool {
        self.error_tracking_standalone_enabled
    }

    /// Returns whether the rare sampler is enabled.
    pub const fn rare_sampler_enabled(&self) -> bool {
        self.rare_sampler_enabled
    }

    /// Returns rare sampler target traces per second.
    pub const fn rare_sampler_tps(&self) -> f64 {
        self.rare_sampler_tps
    }

    /// Returns rare sampler cooldown period in seconds.
    pub const fn rare_sampler_cooldown_period_secs(&self) -> f64 {
        self.rare_sampler_cooldown_period_secs
    }

    /// Returns rare sampler cardinality.
    pub const fn rare_sampler_cardinality(&self) -> usize {
        self.rare_sampler_cardinality
    }

    /// Returns the default trace environment.
    pub fn default_env(&self) -> &str {
        &self.default_env
    }

    /// Returns normalized OTLP sampling rate.
    pub const fn otlp_sampling_rate(&self) -> f64 {
        self.otlp_sampling_rate
    }
}

/// Native Datadog APM stats encoder settings.
#[derive(Clone, Debug)]
pub struct DatadogApmStatsEncoderConfiguration {
    flush_timeout_secs: u64,
    env: String,
}

impl DatadogApmStatsEncoderConfiguration {
    /// Creates native Datadog APM stats encoder settings.
    pub fn new(flush_timeout_secs: u64, env: String) -> Self {
        Self {
            flush_timeout_secs,
            env,
        }
    }

    /// Returns the flush timeout in seconds.
    pub const fn flush_timeout_secs(&self) -> u64 {
        self.flush_timeout_secs
    }

    /// Returns the default stats environment.
    pub fn env(&self) -> &str {
        &self.env
    }
}

/// Native Datadog metrics encoder settings.
#[derive(Clone, Debug)]
pub struct DatadogMetricsEncoderConfiguration {
    max_metrics_per_payload: usize,
    max_payload_size: usize,
    max_uncompressed_payload_size: usize,
    max_series_payload_size: usize,
    max_series_uncompressed_payload_size: usize,
    max_series_points_per_payload: usize,
    flush_timeout_secs: u64,
    compressor_kind: String,
    zstd_compressor_level: i32,
    use_v2_api_series: bool,
    log_payloads: bool,
}

impl DatadogMetricsEncoderConfiguration {
    /// Creates native Datadog metrics encoder settings.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        max_metrics_per_payload: usize, max_payload_size: usize, max_uncompressed_payload_size: usize,
        max_series_payload_size: usize, max_series_uncompressed_payload_size: usize,
        max_series_points_per_payload: usize, flush_timeout_secs: u64, compressor_kind: String,
        zstd_compressor_level: i32, use_v2_api_series: bool, log_payloads: bool,
    ) -> Self {
        Self {
            max_metrics_per_payload,
            max_payload_size,
            max_uncompressed_payload_size,
            max_series_payload_size,
            max_series_uncompressed_payload_size,
            max_series_points_per_payload,
            flush_timeout_secs,
            compressor_kind,
            zstd_compressor_level,
            use_v2_api_series,
            log_payloads,
        }
    }

    /// Returns the maximum number of metrics per payload.
    pub const fn max_metrics_per_payload(&self) -> usize {
        self.max_metrics_per_payload
    }

    /// Returns the generic compressed payload limit in bytes.
    pub const fn max_payload_size(&self) -> usize {
        self.max_payload_size
    }

    /// Returns the generic uncompressed payload limit in bytes.
    pub const fn max_uncompressed_payload_size(&self) -> usize {
        self.max_uncompressed_payload_size
    }

    /// Returns the V2 series compressed payload limit in bytes.
    pub const fn max_series_payload_size(&self) -> usize {
        self.max_series_payload_size
    }

    /// Returns the V2 series uncompressed payload limit in bytes.
    pub const fn max_series_uncompressed_payload_size(&self) -> usize {
        self.max_series_uncompressed_payload_size
    }

    /// Returns the maximum number of V2 series points per payload.
    pub const fn max_series_points_per_payload(&self) -> usize {
        self.max_series_points_per_payload
    }

    /// Returns the flush timeout in seconds.
    pub const fn flush_timeout_secs(&self) -> u64 {
        self.flush_timeout_secs
    }

    /// Returns the serializer compressor kind.
    pub fn compressor_kind(&self) -> &str {
        &self.compressor_kind
    }

    /// Returns the zstd compression level.
    pub const fn zstd_compressor_level(&self) -> i32 {
        self.zstd_compressor_level
    }

    /// Returns whether to use the V2 series API.
    pub const fn use_v2_api_series(&self) -> bool {
        self.use_v2_api_series
    }

    /// Returns whether encoded metrics should be logged.
    pub const fn log_payloads(&self) -> bool {
        self.log_payloads
    }
}

/// Native DogStatsD debug-log destination settings.
#[derive(Clone, Debug)]
pub struct DogStatsDDebugLogConfiguration {
    metrics_stats_enabled: DynamicValue<bool>,
    logging_enabled: bool,
    log_file: PathBuf,
    log_file_max_size_bytes: u64,
    log_file_max_rolls: usize,
}

impl DogStatsDDebugLogConfiguration {
    /// Creates native DogStatsD debug-log destination settings.
    pub fn new(
        metrics_stats_enabled: DynamicValue<bool>, logging_enabled: bool, log_file: PathBuf,
        log_file_max_size_bytes: u64, log_file_max_rolls: usize,
    ) -> Self {
        Self {
            metrics_stats_enabled,
            logging_enabled,
            log_file,
            log_file_max_size_bytes,
            log_file_max_rolls,
        }
    }

    /// Returns whether metric-level DogStatsD statistics are currently enabled.
    pub fn metrics_stats_enabled(&self) -> DynamicValue<bool> {
        self.metrics_stats_enabled.clone()
    }

    /// Returns whether the debug-log destination should be added to the topology.
    pub const fn logging_enabled(&self) -> bool {
        self.logging_enabled
    }

    /// Returns the DogStatsD debug log file path.
    pub fn log_file(&self) -> &PathBuf {
        &self.log_file
    }

    /// Returns maximum active log-file size in bytes.
    pub const fn log_file_max_size_bytes(&self) -> u64 {
        self.log_file_max_size_bytes
    }

    /// Returns the number of rolled log files to keep.
    pub const fn log_file_max_rolls(&self) -> usize {
        self.log_file_max_rolls
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn cargo_toml_stays_leaf_like() {
        let manifest = include_str!("../Cargo.toml");

        assert!(!manifest.contains("datadog-agent-config"));
        assert!(!manifest.contains("saluki-config"));
        assert!(!manifest.contains("agent-data-plane-config"));
    }
}
