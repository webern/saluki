//! Native Saluki configuration assembled by the translator.
//!
//! `TotalSalukiConfiguration` is the front-of-facade native config model for ADP. It is described
//! in Saluki terms, not Datadog key names: the translator (`crate::translator`) is the only place
//! that knows Datadog key paths, serde renames, and schema fixups. Every supported Datadog key has
//! exactly one destination field here.
//!
//! The structs are intentionally thin and std/primitive-typed. Rich Saluki types (listen addresses,
//! byte sizes, durations) are deferred to the per-component cutover PRs; narrowing the schema's
//! faithful `f64`/`String` mirror into those types belongs with the component that consumes them.
//!
//! Several opaque schema leaves (`Option<serde_json::Value>` for `dd_url`, `site`, `bind_host`,
//! `dogstatsd_mapper_profiles`, the MRF overrides) are carried raw for now. The schema does not
//! decompose them into typed sub-fields, so the translator stores them verbatim and a later
//! component PR decides their shape.

use std::collections::HashMap;

use serde_json::Value;

/// The complete native Saluki configuration produced by translating `DatadogConfiguration`.
///
/// Translation into this type is lossy with respect to absent-vs-explicit-default: an absent
/// Datadog key and one set to its schema default (or the empty-string "unset" sentinel) both
/// resolve to the same native value here. Logic that must observe explicit presence (the PR 10
/// dynamic-config diff) must operate on `DatadogConfiguration`, not this type. See
/// [`crate::translator::translate`] for the full invariant.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TotalSalukiConfiguration {
    /// ADP data-plane-specific tuning.
    pub data_plane: DataPlaneConfig,
    /// DogStatsD listener, decoding, and debug-logging configuration.
    pub dogstatsd: DogStatsDConfig,
    /// Outbound Datadog delivery (forwarder, endpoints, proxy, TLS, API key).
    pub forwarder: ForwarderConfig,
    /// Metrics pipeline configuration (aggregation, serializer, filterlists, MRF, statsd forward).
    pub metrics: MetricsConfig,
    /// Trace/APM configuration (obfuscation rules, environment).
    pub traces: TracesConfig,
    /// OTLP ingest configuration.
    pub otlp: OtlpConfig,
    /// Logging configuration (process logging format/destinations).
    pub logs: LogsConfig,
    /// Settings that span multiple subsystems or ADP as a whole.
    pub cross_cutting: CrossCuttingConfig,
}

/// TLS protocol version negotiated for outbound connections.
///
/// ADP (rustls) supports only 1.2 and 1.3. The translator clamps `tlsv1.0`/`tlsv1.1` to
/// [`TlsVersion::Tls12`] and maps unrecognized values to the schema default of 1.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TlsVersion {
    /// TLS 1.2 (also the clamp target for the unsupported 1.0/1.1 values).
    #[default]
    Tls12,
    /// TLS 1.3.
    Tls13,
}

/// ADP data-plane-specific tuning knobs delivered under `data_plane.*`.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct DataPlaneConfig {
    /// `data_plane.dogstatsd.aggregator_tag_filter_cache_capacity`: per-context dedup cache size for
    /// the tag filterlist.
    pub aggregator_tag_filter_cache_capacity: u64,
}

/// DogStatsD listener, decoding, and debug-logging configuration.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct DogStatsDConfig {
    /// `dogstatsd_port`: UDP listen port (narrowed from the schema's `f64`).
    pub port: u16,
    /// `bind_host`: global listen host fallback (opaque schema value carried raw).
    pub bind_host: Option<Value>,
    /// `dogstatsd_non_local_traffic`: accept non-localhost UDP/TCP.
    pub non_local_traffic: bool,
    /// `dogstatsd_socket`: UDS datagram socket path (`None`/empty disables).
    pub socket: Option<String>,
    /// `dogstatsd_stream_socket`: UDS stream socket path.
    pub stream_socket: String,
    /// `dogstatsd_stream_log_too_big`: log oversized UDS stream frames.
    pub stream_log_too_big: bool,
    /// `dogstatsd_buffer_size`: receive buffer size in bytes.
    pub buffer_size: u64,
    /// `dogstatsd_so_rcvbuf`: socket receive buffer size in bytes (0 = OS default).
    pub so_rcvbuf: u64,
    /// `dogstatsd_string_interner_size`: interner capacity.
    pub string_interner_size: u64,
    /// `dogstatsd_context_expiry_seconds`: context cache TTL in seconds.
    pub context_expiry_seconds: u64,
    /// `dogstatsd_eol_required`: transports requiring newline-terminated messages.
    pub eol_required: Vec<String>,
    /// `dogstatsd_tag_cardinality`: tag cardinality level (low/orchestrator/high).
    pub tag_cardinality: String,
    /// `dogstatsd_tags`: extra tags appended to all received metrics.
    pub tags: Vec<String>,
    /// `dogstatsd_entity_id_precedence`: prefer client entity-ID over auto-detection.
    pub entity_id_precedence: bool,
    /// `dogstatsd_origin_detection`: enable UDS origin detection.
    pub origin_detection: bool,
    /// `dogstatsd_origin_detection_client`: honor client-provided origin proto fields.
    pub origin_detection_client: bool,
    /// `dogstatsd_origin_optout_enabled`: allow clients to opt out of origin enrichment.
    pub origin_optout_enabled: bool,
    /// `origin_detection_unified`: unified origin detection across protocols.
    pub origin_detection_unified: bool,
    /// `provider_kind`: workload-meta provider kind (opaque schema value carried raw).
    pub provider_kind: Option<Value>,
    /// `dogstatsd_capture_path`: traffic capture file location.
    pub capture_path: String,
    /// `dogstatsd_capture_depth`: traffic capture channel depth.
    pub capture_depth: u64,
    /// `dogstatsd_mapper_profiles`: metric-name mapping profile definitions (opaque, carried raw).
    pub mapper_profiles: Option<Value>,
    /// `dogstatsd_mapper_cache_size`: mapper result LRU cache size. Partial: `0` disables only the
    /// cache (profiles still run), unlike the core Agent where `0` disables the mapper entirely.
    pub mapper_cache_size: u64,
    /// `dogstatsd_logging_enabled`: enable the DogStatsD metric debug log destination.
    pub logging_enabled: bool,
    /// `dogstatsd_metrics_stats_enable`: collect per-metric debug stats. Partial (warns on
    /// non-default).
    pub metrics_stats_enable: bool,
    /// `dogstatsd_log_file`: dedicated debug-log file path.
    pub log_file: String,
    /// `dogstatsd_log_file_max_size`: max debug-log file size (size string, for example `10Mb`).
    pub log_file_max_size: String,
    /// `dogstatsd_log_file_max_rolls`: max debug-log roll count.
    pub log_file_max_rolls: u64,
}

/// Outbound Datadog delivery configuration: forwarder tuning, endpoint resolution, proxy, TLS, and
/// the API key.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ForwarderConfig {
    /// `api_key`: API key used to authenticate to the Datadog intake.
    pub api_key: String,
    /// `dd_url`: intake endpoint URL override (opaque schema value carried raw).
    pub dd_url: Option<Value>,
    /// `site`: Datadog site (opaque schema value carried raw).
    pub site: Option<Value>,
    /// `additional_endpoints`: dual-ship targets, endpoint URL -> API keys.
    pub additional_endpoints: HashMap<String, Vec<String>>,
    /// `allow_arbitrary_tags`: relax backend tag validation via HTTP header.
    pub allow_arbitrary_tags: bool,
    /// `use_v2_api_series`: submit series via the v2 intake API.
    pub use_v2_api_series: bool,
    /// `forwarder_num_workers`: forwarder worker count. Partial: ADP may treat this differently
    /// from the core Agent's worker model.
    pub num_workers: u64,
    /// `forwarder_high_prio_buffer_size`: high-priority buffer depth. Partial.
    pub high_prio_buffer_size: u64,
    /// `forwarder_max_concurrent_requests`: per-worker in-flight request cap.
    pub max_concurrent_requests: u64,
    /// `forwarder_timeout`: request timeout in seconds.
    pub timeout_secs: u64,
    /// `forwarder_connection_reset_interval`: connection reset interval in seconds (0 = never).
    pub connection_reset_interval_secs: u64,
    /// `forwarder_backoff_base`: exponential backoff base.
    pub backoff_base: f64,
    /// `forwarder_backoff_factor`: exponential backoff growth factor.
    pub backoff_factor: f64,
    /// `forwarder_backoff_max`: maximum backoff wait in seconds.
    pub backoff_max: f64,
    /// `forwarder_recovery_interval`: recovery probe interval.
    pub recovery_interval: u64,
    /// `forwarder_recovery_reset`: reset the recovery interval on success.
    pub recovery_reset: bool,
    /// `forwarder_retry_queue_max_size`: in-memory retry queue cap (transactions).
    pub retry_queue_max_size: u64,
    /// `forwarder_retry_queue_payloads_max_size`: in-memory retry queue cap (bytes).
    pub retry_queue_payloads_max_size: u64,
    /// `forwarder_storage_path`: on-disk retry storage directory.
    pub storage_path: String,
    /// `forwarder_storage_max_size_in_bytes`: on-disk retry storage cap in bytes (0 = disabled).
    pub storage_max_size_in_bytes: u64,
    /// `forwarder_storage_max_disk_ratio`: on-disk retry storage cap as a disk-usage ratio.
    pub storage_max_disk_ratio: f64,
    /// `forwarder_outdated_file_in_days`: discard retry files older than this many days.
    pub outdated_file_in_days: u64,
    /// `forwarder_http_protocol`: outbound HTTP protocol (`auto` or `http1`).
    pub http_protocol: String,
    /// `proxy.http` / `proxy.https` / `proxy.no_proxy`: assembled proxy configuration.
    pub proxy: ProxyConfig,
    /// `no_proxy_nonexact_match`: enable flexible `no_proxy` matching.
    pub no_proxy_nonexact_match: bool,
    /// `use_proxy_for_cloud_metadata`: keep cloud-provider IPs out of the auto `no_proxy` list.
    pub use_proxy_for_cloud_metadata: bool,
    /// `min_tls_version`: minimum negotiated TLS version. Partial: 1.0/1.1 clamp to 1.2.
    pub min_tls_version: TlsVersion,
    /// `skip_ssl_validation`: skip TLS certificate validation. Partial.
    pub skip_ssl_validation: bool,
}

/// HTTP/HTTPS proxy configuration, assembled from the three `proxy.*` leaves.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ProxyConfig {
    /// `proxy.http`: proxy URL for HTTP requests (empty = none).
    pub http: Option<String>,
    /// `proxy.https`: proxy URL for HTTPS requests (empty = none).
    pub https: Option<String>,
    /// `proxy.no_proxy`: hosts that bypass the proxy.
    pub no_proxy: Vec<String>,
}

/// Metrics pipeline configuration: serializer limits, aggregation behavior, histogram and
/// filterlist settings, statsd forwarding, multi-region failover, and forwarding integrations.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct MetricsConfig {
    /// `serializer_compressor_kind`: payload compression algorithm (for example `zstd`).
    pub serializer_compressor_kind: String,
    /// `serializer_zstd_compressor_level`: zstd level. Partial: ADP's zstd level range may differ
    /// from the core Agent's.
    pub serializer_zstd_compressor_level: i32,
    /// `serializer_max_payload_size`: max compressed payload size in bytes.
    pub serializer_max_payload_size: u64,
    /// `serializer_max_uncompressed_payload_size`: max uncompressed payload size in bytes.
    pub serializer_max_uncompressed_payload_size: u64,
    /// `serializer_max_series_payload_size`: max compressed series payload size in bytes.
    pub serializer_max_series_payload_size: u64,
    /// `serializer_max_series_uncompressed_payload_size`: max uncompressed series payload bytes.
    pub serializer_max_series_uncompressed_payload_size: u64,
    /// `serializer_max_series_points_per_payload`: max points per series payload.
    pub serializer_max_series_points_per_payload: u64,
    /// `enable_payloads.series`: emit series payloads.
    pub enable_series: bool,
    /// `enable_payloads.events`: emit event payloads.
    pub enable_events: bool,
    /// `enable_payloads.service_checks`: emit service-check payloads.
    pub enable_service_checks: bool,
    /// `enable_payloads.sketches`: emit sketch (distribution) payloads.
    pub enable_sketches: bool,
    /// `dogstatsd_no_aggregation_pipeline`: enable the no-aggregation timestamped path.
    pub no_aggregation_pipeline: bool,
    /// `dogstatsd_flush_incomplete_buckets`: flush open aggregation buckets on shutdown.
    pub flush_incomplete_buckets: bool,
    /// `histogram_aggregates`: which aggregates to compute for histograms.
    pub histogram_aggregates: Vec<String>,
    /// `histogram_copy_to_distribution`: also emit histograms as distributions.
    pub histogram_copy_to_distribution: bool,
    /// `histogram_copy_to_distribution_prefix`: prefix for copied distribution metrics.
    pub histogram_copy_to_distribution_prefix: String,
    /// `metric_filterlist`: post-aggregate metric-name filterlist.
    pub metric_filterlist: Vec<String>,
    /// `metric_filterlist_match_prefix`: treat filterlist entries as prefixes.
    pub metric_filterlist_match_prefix: bool,
    /// `statsd_metric_namespace`: namespace prefix for all received statsd metrics.
    pub statsd_metric_namespace: String,
    /// `statsd_metric_namespace_blacklist`: namespaces exempt from the prefix.
    pub statsd_metric_namespace_blacklist: Vec<String>,
    /// `statsd_metric_blocklist`: dropped metric names.
    pub statsd_metric_blocklist: Vec<String>,
    /// `statsd_metric_blocklist_match_prefix`: treat blocklist entries as prefixes.
    pub statsd_metric_blocklist_match_prefix: bool,
    /// `statsd_forward_host`: forward raw statsd packets to this host. Partial: empty disables.
    pub statsd_forward_host: Option<String>,
    /// `statsd_forward_port`: port for `statsd_forward_host`.
    pub statsd_forward_port: u16,
    /// Multi-region failover configuration.
    pub multi_region_failover: MultiRegionFailoverConfig,
    /// `observability_pipelines_worker.metrics.*`: OPW metrics forwarding.
    pub observability_pipelines_worker_metrics: ForwardingIntegration,
    /// `vector.metrics.*`: Vector metrics forwarding.
    pub vector_metrics: ForwardingIntegration,
}

/// Multi-region failover metric routing (`multi_region_failover.*`).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct MultiRegionFailoverConfig {
    /// `multi_region_failover.enabled`: enable MRF. Partial: ADP supports metric failover only.
    pub enabled: bool,
    /// `multi_region_failover.failover_metrics`: route metrics to the failover region.
    pub failover_metrics: bool,
    /// `multi_region_failover.api_key`: failover-region API key (opaque, carried raw).
    pub api_key: Option<Value>,
    /// `multi_region_failover.dd_url`: failover-region intake URL (opaque, carried raw).
    pub dd_url: Option<Value>,
    /// `multi_region_failover.site`: failover-region site (opaque, carried raw).
    pub site: Option<Value>,
    /// `multi_region_failover.metric_allowlist`: metrics allowed to fail over (opaque, carried raw).
    pub metric_allowlist: Option<Value>,
}

/// A metrics-forwarding integration toggle plus its endpoint, shared by the OPW and Vector leaves.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ForwardingIntegration {
    /// Whether forwarding to this integration is enabled.
    pub enabled: bool,
    /// Destination URL for forwarded metrics.
    pub url: String,
}

/// Trace/APM configuration: span obfuscation rules and the deployment environment.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TracesConfig {
    /// `env`: deployment environment tag attached to emitted telemetry.
    pub env: String,
    /// `log_payloads`: log raw payloads for debugging (cross-pipeline; stored here).
    pub log_payloads: bool,
    /// Span obfuscation rules (`apm_config.obfuscation.*`).
    pub obfuscation: ObfuscationConfig,
}

/// Per-backend trace obfuscation rules (`apm_config.obfuscation.*`).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ObfuscationConfig {
    /// Credit-card obfuscation.
    pub credit_cards: CreditCardObfuscation,
    /// HTTP URL/query obfuscation.
    pub http: HttpObfuscation,
    /// Elasticsearch query obfuscation.
    pub elasticsearch: SqlBackendObfuscation,
    /// MongoDB query obfuscation.
    pub mongodb: SqlBackendObfuscation,
    /// OpenSearch query obfuscation.
    pub opensearch: SqlBackendObfuscation,
    /// Memcached command obfuscation.
    pub memcached: MemcachedObfuscation,
    /// Redis command obfuscation.
    pub redis: KvCommandObfuscation,
    /// Valkey command obfuscation.
    pub valkey: KvCommandObfuscation,
}

/// `apm_config.obfuscation.credit_cards.*`.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CreditCardObfuscation {
    /// Enable credit-card obfuscation.
    pub enabled: bool,
    /// Keys whose values are never obfuscated.
    pub keep_values: Vec<String>,
    /// Apply a Luhn checksum to reduce false positives.
    pub luhn: bool,
}

/// `apm_config.obfuscation.http.*`.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct HttpObfuscation {
    /// Replace numeric path segments with `?`.
    pub remove_paths_with_digits: bool,
    /// Obfuscate query strings.
    pub remove_query_string: bool,
}

/// Obfuscation rules for the SQL-like backends (Elasticsearch, MongoDB, OpenSearch).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SqlBackendObfuscation {
    /// Enable obfuscation for this backend.
    pub enabled: bool,
    /// Keys whose values are never obfuscated.
    pub keep_values: Vec<String>,
    /// Keys whose values are passed through SQL obfuscation.
    pub obfuscate_sql_values: Vec<String>,
}

/// `apm_config.obfuscation.memcached.*`.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct MemcachedObfuscation {
    /// Enable Memcached obfuscation.
    pub enabled: bool,
    /// Keep the full command (including lookup keys).
    pub keep_command: bool,
}

/// Obfuscation rules for key/value command stores (Redis, Valkey).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct KvCommandObfuscation {
    /// Enable obfuscation for this store.
    pub enabled: bool,
    /// Replace all command arguments with a single `?`.
    pub remove_all_args: bool,
}

/// OTLP ingest configuration (`otlp_config.*`).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct OtlpConfig {
    /// `otlp_config.metrics.enabled`: accept OTLP metrics.
    pub metrics_enabled: bool,
    /// `otlp_config.logs.enabled`: accept OTLP logs.
    pub logs_enabled: bool,
    /// `otlp_config.traces.enabled`: accept OTLP traces.
    pub traces_enabled: bool,
    /// `otlp_config.traces.internal_port`: internal trace forwarding port.
    pub traces_internal_port: u16,
    /// `otlp_config.traces.probabilistic_sampler.sampling_percentage`: ingestion sampling percent.
    pub traces_sampling_percentage: f64,
    /// gRPC receiver settings.
    pub grpc: OtlpGrpcConfig,
    /// HTTP receiver settings.
    pub http: OtlpHttpConfig,
}

/// `otlp_config.receiver.protocols.grpc.*`.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct OtlpGrpcConfig {
    /// gRPC listener endpoint.
    pub endpoint: String,
    /// gRPC transport (`tcp`, `unix`, ...).
    pub transport: String,
    /// Max accepted message size in MiB (0 = grpc-go default of 4 MiB).
    pub max_recv_msg_size_mib: u64,
}

/// `otlp_config.receiver.protocols.http.*`.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct OtlpHttpConfig {
    /// HTTP listener endpoint.
    pub endpoint: String,
}

/// Logging configuration (process log format and remote destinations).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct LogsConfig {
    /// `log_level`: minimum process log level. Partial: ADP maps the Agent's level set onto its
    /// own filter, which may not honor every Agent-specific level identically.
    pub log_level: String,
    /// `log_format_rfc3339`: emit RFC3339 timestamps.
    pub log_format_rfc3339: bool,
    /// `syslog_rfc`: emit RFC 5424 syslog format.
    pub syslog_rfc: bool,
    /// `syslog_uri`: remote syslog URI (empty = local domain socket).
    pub syslog_uri: String,
}

/// Settings that span multiple subsystems or ADP startup as a whole.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CrossCuttingConfig {
    /// `cmd_port`: Datadog Agent IPC/CMD API port.
    pub cmd_port: u16,
    /// `vsock_addr`: vsock address for the Agent IPC connection.
    pub vsock_addr: String,
    /// `cri_connection_timeout`: CRI runtime connection timeout in seconds.
    pub cri_connection_timeout_secs: u64,
    /// `cri_query_timeout`: CRI runtime query timeout in seconds.
    pub cri_query_timeout_secs: u64,
}
