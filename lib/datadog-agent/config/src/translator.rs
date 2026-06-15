//! Hand-written translation from the generated `DatadogConfiguration` into native
//! `TotalSalukiConfiguration`.
//!
//! `Translator` implements the generated [`DatadogConfigConsumer`] witness, so the compiler forces a
//! destination for every supported Datadog key: if the overlay gains or loses a `support: full` /
//! `support: partial` key, the witness trait changes and this impl fails to build until the new
//! shape is handled. Each `consume_*` method performs only semantic translation -- narrowing the
//! schema's faithful `f64`/`String` mirror, clamping, unit/enum parsing, and grouping multi-leaf
//! settings (proxy) into native sub-structs. Defaults are already baked into `DatadogConfiguration`
//! and threaded through `drive`, so the methods never re-supply default literals.
//!
//! This module is the sole owner of Datadog key semantics; `TotalSalukiConfiguration` carries no
//! Datadog key names.

use serde_json::Value;

use crate::total_config::{TlsVersion, TotalSalukiConfiguration};
use crate::witness::{drive, DatadogConfigConsumer};
use crate::DatadogConfiguration;

/// Translate a typed `DatadogConfiguration` into native `TotalSalukiConfiguration`.
///
/// Runs the generated [`drive`] over a fresh [`Translator`], then finalizes any accumulated state.
///
/// # Invariant: translation is lossy w.r.t. absent vs. explicit-default
///
/// [`drive`] resolves an absent `Option<Section>` to its schema default via
/// `.clone().unwrap_or_default()`, and several `consume_*` methods fold the schema's `""` "unset"
/// sentinel into `None`. Both collapse "the key was absent" and "the key was set to its
/// default/empty value" onto the same native value, so the translated `TotalSalukiConfiguration`
/// cannot distinguish the two. This is intentional: the native model carries values, not presence.
///
/// Consequently, any logic that must observe whether a key was *explicitly* set -- in particular
/// the dynamic-config diffing planned for PR 10 -- must diff at the typed [`DatadogConfiguration`]
/// layer (where absence is still an `Option::None`), NOT against this translated output.
pub fn translate(config: &DatadogConfiguration) -> TotalSalukiConfiguration {
    let mut translator = Translator::new();
    drive(config, &mut translator);
    translator.finish()
}

/// Accumulating translation state.
///
/// Most leaves write straight into `config`. State that is assembled from multiple leaves (the
/// three `proxy.*` keys) is staged here and merged in [`Translator::finish`].
pub struct Translator {
    config: TotalSalukiConfiguration,
}

impl Translator {
    /// Create a translator over a default native configuration.
    pub fn new() -> Self {
        Self {
            config: TotalSalukiConfiguration::default(),
        }
    }

    /// Consume the accumulated state and return the native configuration.
    ///
    /// Proxy fields are assembled in place by the `consume_proxy_*` methods, so there is no
    /// deferred merge to perform here today; `finish` exists as the documented assembly point for
    /// future multi-leaf groupings.
    pub fn finish(self) -> TotalSalukiConfiguration {
        self.config
    }
}

impl Default for Translator {
    fn default() -> Self {
        Self::new()
    }
}

/// Narrow a schema `f64` to `u16`, saturating out-of-range values into the `u16` bounds.
///
/// Ports and similar small counts are typed `number` in the schema (hence `f64` after typify) but
/// are conceptually unsigned 16-bit. Negative or oversized values are clamped rather than wrapped.
///
/// Like the other `f64_to_*` helpers, the fractional part is truncated toward zero (an `as` cast),
/// and values outside the target type's range are saturated at its bounds; both are intentional.
fn f64_to_u16(value: f64) -> u16 {
    if value.is_nan() || value <= 0.0 {
        0
    } else if value >= u16::MAX as f64 {
        u16::MAX
    } else {
        value as u16
    }
}

/// Narrow a schema `f64` to `u64`, saturating negatives/NaN to 0 and oversized values to `u64::MAX`.
///
/// The fractional part is truncated toward zero; out-of-range values are saturated at the bounds.
fn f64_to_u64(value: f64) -> u64 {
    if value.is_nan() || value <= 0.0 {
        0
    } else if value >= u64::MAX as f64 {
        u64::MAX
    } else {
        value as u64
    }
}

/// Narrow a schema `f64` to `i32` (signed levels such as the zstd compressor level), saturating.
///
/// The fractional part is truncated toward zero; values below `i32::MIN` or above `i32::MAX` are
/// saturated at the respective bound rather than wrapping.
fn f64_to_i32(value: f64) -> i32 {
    if value.is_nan() {
        0
    } else if value >= i32::MAX as f64 {
        i32::MAX
    } else if value <= i32::MIN as f64 {
        i32::MIN
    } else {
        value as i32
    }
}

/// Map a non-empty string to `Some`, an empty string to `None`.
///
/// Several schema strings use `""` as the "unset" sentinel (proxy URLs, statsd forward host). The
/// native model represents "unset" as `None` instead.
fn non_empty(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

impl DatadogConfigConsumer for Translator {
    // --- cross-cutting ------------------------------------------------------------------------

    fn consume_cmd_port(&mut self, value: f64) {
        self.config.cross_cutting.cmd_port = f64_to_u16(value);
    }

    fn consume_vsock_addr(&mut self, value: String) {
        self.config.cross_cutting.vsock_addr = value;
    }

    fn consume_cri_connection_timeout(&mut self, value: f64) {
        self.config.cross_cutting.cri_connection_timeout_secs = f64_to_u64(value);
    }

    fn consume_cri_query_timeout(&mut self, value: f64) {
        self.config.cross_cutting.cri_query_timeout_secs = f64_to_u64(value);
    }

    // --- data plane ---------------------------------------------------------------------------

    fn consume_data_plane_dogstatsd_aggregator_tag_filter_cache_capacity(&mut self, value: f64) {
        self.config.data_plane.aggregator_tag_filter_cache_capacity = f64_to_u64(value);
    }

    // --- DogStatsD ----------------------------------------------------------------------------

    fn consume_dogstatsd_port(&mut self, value: f64) {
        self.config.dogstatsd.port = f64_to_u16(value);
    }

    fn consume_bind_host(&mut self, value: Option<Value>) {
        self.config.dogstatsd.bind_host = value;
    }

    fn consume_dogstatsd_non_local_traffic(&mut self, value: bool) {
        self.config.dogstatsd.non_local_traffic = value;
    }

    fn consume_dogstatsd_socket(&mut self, value: Option<String>) {
        // An explicitly-empty socket path means "disabled", same as absence.
        self.config.dogstatsd.socket = value.and_then(non_empty);
    }

    fn consume_dogstatsd_stream_socket(&mut self, value: String) {
        self.config.dogstatsd.stream_socket = value;
    }

    fn consume_dogstatsd_stream_log_too_big(&mut self, value: bool) {
        self.config.dogstatsd.stream_log_too_big = value;
    }

    fn consume_dogstatsd_buffer_size(&mut self, value: f64) {
        self.config.dogstatsd.buffer_size = f64_to_u64(value);
    }

    fn consume_dogstatsd_so_rcvbuf(&mut self, value: f64) {
        self.config.dogstatsd.so_rcvbuf = f64_to_u64(value);
    }

    fn consume_dogstatsd_string_interner_size(&mut self, value: f64) {
        self.config.dogstatsd.string_interner_size = f64_to_u64(value);
    }

    fn consume_dogstatsd_context_expiry_seconds(&mut self, value: f64) {
        self.config.dogstatsd.context_expiry_seconds = f64_to_u64(value);
    }

    fn consume_dogstatsd_eol_required(&mut self, value: Vec<String>) {
        self.config.dogstatsd.eol_required = value;
    }

    fn consume_dogstatsd_tag_cardinality(&mut self, value: String) {
        self.config.dogstatsd.tag_cardinality = value;
    }

    fn consume_dogstatsd_tags(&mut self, value: Vec<String>) {
        self.config.dogstatsd.tags = value;
    }

    fn consume_dogstatsd_entity_id_precedence(&mut self, value: bool) {
        self.config.dogstatsd.entity_id_precedence = value;
    }

    fn consume_dogstatsd_origin_detection(&mut self, value: bool) {
        self.config.dogstatsd.origin_detection = value;
    }

    fn consume_dogstatsd_origin_detection_client(&mut self, value: bool) {
        self.config.dogstatsd.origin_detection_client = value;
    }

    fn consume_dogstatsd_origin_optout_enabled(&mut self, value: bool) {
        self.config.dogstatsd.origin_optout_enabled = value;
    }

    fn consume_origin_detection_unified(&mut self, value: bool) {
        self.config.dogstatsd.origin_detection_unified = value;
    }

    fn consume_provider_kind(&mut self, value: Option<Value>) {
        self.config.dogstatsd.provider_kind = value;
    }

    fn consume_dogstatsd_capture_path(&mut self, value: String) {
        self.config.dogstatsd.capture_path = value;
    }

    fn consume_dogstatsd_capture_depth(&mut self, value: f64) {
        self.config.dogstatsd.capture_depth = f64_to_u64(value);
    }

    fn consume_dogstatsd_mapper_profiles(&mut self, value: Option<Value>) {
        self.config.dogstatsd.mapper_profiles = value;
    }

    fn consume_dogstatsd_mapper_cache_size(&mut self, value: f64) {
        // Partial support: in ADP `0` disables only the result cache (mapping profiles still run),
        // whereas the core Agent's LRU rejects `0` and disables the whole mapper. We carry the
        // value through verbatim; the divergent zero semantics live in the mapper component.
        self.config.dogstatsd.mapper_cache_size = f64_to_u64(value);
    }

    fn consume_dogstatsd_logging_enabled(&mut self, value: bool) {
        self.config.dogstatsd.logging_enabled = value;
    }

    fn consume_dogstatsd_metrics_stats_enable(&mut self, value: bool) {
        // Partial support: ADP exposes per-metric stats on demand rather than via the core Agent's
        // always-on stats endpoint; the flag still gates the debug-log destination.
        self.config.dogstatsd.metrics_stats_enable = value;
    }

    fn consume_dogstatsd_log_file(&mut self, value: String) {
        self.config.dogstatsd.log_file = value;
    }

    fn consume_dogstatsd_log_file_max_size(&mut self, value: String) {
        self.config.dogstatsd.log_file_max_size = value;
    }

    fn consume_dogstatsd_log_file_max_rolls(&mut self, value: f64) {
        self.config.dogstatsd.log_file_max_rolls = f64_to_u64(value);
    }

    // --- forwarder / endpoints / proxy / TLS --------------------------------------------------

    fn consume_api_key(&mut self, value: String) {
        self.config.forwarder.api_key = value;
    }

    fn consume_dd_url(&mut self, value: Option<Value>) {
        self.config.forwarder.dd_url = value;
    }

    fn consume_site(&mut self, value: Option<Value>) {
        self.config.forwarder.site = value;
    }

    fn consume_additional_endpoints(&mut self, value: std::collections::HashMap<String, Vec<String>>) {
        self.config.forwarder.additional_endpoints = value;
    }

    fn consume_allow_arbitrary_tags(&mut self, value: bool) {
        self.config.forwarder.allow_arbitrary_tags = value;
    }

    fn consume_use_v2_api_series(&mut self, value: bool) {
        self.config.forwarder.use_v2_api_series = value;
    }

    fn consume_forwarder_num_workers(&mut self, value: f64) {
        // Partial support: ADP's worker model differs from the core Agent's; the count is carried
        // through and interpreted by the forwarder component.
        self.config.forwarder.num_workers = f64_to_u64(value);
    }

    fn consume_forwarder_high_prio_buffer_size(&mut self, value: f64) {
        // Partial support: buffer sizing semantics differ in ADP's pipeline.
        self.config.forwarder.high_prio_buffer_size = f64_to_u64(value);
    }

    fn consume_forwarder_max_concurrent_requests(&mut self, value: f64) {
        self.config.forwarder.max_concurrent_requests = f64_to_u64(value);
    }

    fn consume_forwarder_timeout(&mut self, value: f64) {
        self.config.forwarder.timeout_secs = f64_to_u64(value);
    }

    fn consume_forwarder_connection_reset_interval(&mut self, value: f64) {
        self.config.forwarder.connection_reset_interval_secs = f64_to_u64(value);
    }

    fn consume_forwarder_backoff_base(&mut self, value: f64) {
        self.config.forwarder.backoff_base = value;
    }

    fn consume_forwarder_backoff_factor(&mut self, value: f64) {
        self.config.forwarder.backoff_factor = value;
    }

    fn consume_forwarder_backoff_max(&mut self, value: f64) {
        self.config.forwarder.backoff_max = value;
    }

    fn consume_forwarder_recovery_interval(&mut self, value: f64) {
        self.config.forwarder.recovery_interval = f64_to_u64(value);
    }

    fn consume_forwarder_recovery_reset(&mut self, value: bool) {
        self.config.forwarder.recovery_reset = value;
    }

    fn consume_forwarder_retry_queue_max_size(&mut self, value: f64) {
        self.config.forwarder.retry_queue_max_size = f64_to_u64(value);
    }

    fn consume_forwarder_retry_queue_payloads_max_size(&mut self, value: f64) {
        self.config.forwarder.retry_queue_payloads_max_size = f64_to_u64(value);
    }

    fn consume_forwarder_storage_path(&mut self, value: String) {
        self.config.forwarder.storage_path = value;
    }

    fn consume_forwarder_storage_max_size_in_bytes(&mut self, value: f64) {
        self.config.forwarder.storage_max_size_in_bytes = f64_to_u64(value);
    }

    fn consume_forwarder_storage_max_disk_ratio(&mut self, value: f64) {
        self.config.forwarder.storage_max_disk_ratio = value;
    }

    fn consume_forwarder_outdated_file_in_days(&mut self, value: f64) {
        self.config.forwarder.outdated_file_in_days = f64_to_u64(value);
    }

    fn consume_forwarder_http_protocol(&mut self, value: String) {
        self.config.forwarder.http_protocol = value;
    }

    fn consume_proxy_http(&mut self, value: String) {
        self.config.forwarder.proxy.http = non_empty(value);
    }

    fn consume_proxy_https(&mut self, value: String) {
        self.config.forwarder.proxy.https = non_empty(value);
    }

    fn consume_proxy_no_proxy(&mut self, value: Vec<String>) {
        self.config.forwarder.proxy.no_proxy = value;
    }

    fn consume_no_proxy_nonexact_match(&mut self, value: bool) {
        self.config.forwarder.no_proxy_nonexact_match = value;
    }

    fn consume_use_proxy_for_cloud_metadata(&mut self, value: bool) {
        self.config.forwarder.use_proxy_for_cloud_metadata = value;
    }

    fn consume_min_tls_version(&mut self, value: String) {
        // Partial support: rustls cannot negotiate TLS 1.0/1.1, so those (accepted for config
        // compatibility) are clamped up to 1.2. 1.3 passes through; unrecognized values fall back
        // to the schema default of 1.2. Matching is case-insensitive per the schema documentation;
        // surrounding whitespace is trimmed so e.g. `"tlsv1.3 "` is not silently downgraded.
        self.config.forwarder.min_tls_version = match value.trim().to_ascii_lowercase().as_str() {
            "tlsv1.3" => TlsVersion::Tls13,
            // "tlsv1.0", "tlsv1.1", "tlsv1.2", and any unknown value resolve to TLS 1.2.
            _ => TlsVersion::Tls12,
        };
    }

    fn consume_skip_ssl_validation(&mut self, value: bool) {
        // Partial support: honored for outbound intake connections.
        self.config.forwarder.skip_ssl_validation = value;
    }

    // --- metrics pipeline ---------------------------------------------------------------------

    fn consume_serializer_compressor_kind(&mut self, value: String) {
        self.config.metrics.serializer_compressor_kind = value;
    }

    fn consume_serializer_zstd_compressor_level(&mut self, value: f64) {
        // Partial support: ADP's zstd level range may differ from the core Agent's; the level is
        // carried through as a signed integer and validated by the serializer component.
        self.config.metrics.serializer_zstd_compressor_level = f64_to_i32(value);
    }

    fn consume_serializer_max_payload_size(&mut self, value: f64) {
        self.config.metrics.serializer_max_payload_size = f64_to_u64(value);
    }

    fn consume_serializer_max_uncompressed_payload_size(&mut self, value: f64) {
        self.config.metrics.serializer_max_uncompressed_payload_size = f64_to_u64(value);
    }

    fn consume_serializer_max_series_payload_size(&mut self, value: f64) {
        self.config.metrics.serializer_max_series_payload_size = f64_to_u64(value);
    }

    fn consume_serializer_max_series_uncompressed_payload_size(&mut self, value: f64) {
        self.config.metrics.serializer_max_series_uncompressed_payload_size = f64_to_u64(value);
    }

    fn consume_serializer_max_series_points_per_payload(&mut self, value: f64) {
        self.config.metrics.serializer_max_series_points_per_payload = f64_to_u64(value);
    }

    fn consume_enable_payloads_series(&mut self, value: bool) {
        self.config.metrics.enable_series = value;
    }

    fn consume_enable_payloads_events(&mut self, value: bool) {
        self.config.metrics.enable_events = value;
    }

    fn consume_enable_payloads_service_checks(&mut self, value: bool) {
        self.config.metrics.enable_service_checks = value;
    }

    fn consume_enable_payloads_sketches(&mut self, value: bool) {
        self.config.metrics.enable_sketches = value;
    }

    fn consume_dogstatsd_no_aggregation_pipeline(&mut self, value: bool) {
        self.config.metrics.no_aggregation_pipeline = value;
    }

    fn consume_dogstatsd_flush_incomplete_buckets(&mut self, value: bool) {
        self.config.metrics.flush_incomplete_buckets = value;
    }

    fn consume_histogram_aggregates(&mut self, value: Vec<String>) {
        self.config.metrics.histogram_aggregates = value;
    }

    fn consume_histogram_copy_to_distribution(&mut self, value: bool) {
        self.config.metrics.histogram_copy_to_distribution = value;
    }

    fn consume_histogram_copy_to_distribution_prefix(&mut self, value: String) {
        self.config.metrics.histogram_copy_to_distribution_prefix = value;
    }

    fn consume_metric_filterlist(&mut self, value: Vec<String>) {
        self.config.metrics.metric_filterlist = value;
    }

    fn consume_metric_filterlist_match_prefix(&mut self, value: bool) {
        self.config.metrics.metric_filterlist_match_prefix = value;
    }

    fn consume_statsd_metric_namespace(&mut self, value: String) {
        self.config.metrics.statsd_metric_namespace = value;
    }

    fn consume_statsd_metric_namespace_blacklist(&mut self, value: Vec<String>) {
        self.config.metrics.statsd_metric_namespace_blacklist = value;
    }

    fn consume_statsd_metric_blocklist(&mut self, value: Vec<String>) {
        self.config.metrics.statsd_metric_blocklist = value;
    }

    fn consume_statsd_metric_blocklist_match_prefix(&mut self, value: bool) {
        self.config.metrics.statsd_metric_blocklist_match_prefix = value;
    }

    fn consume_statsd_forward_host(&mut self, value: String) {
        // Partial support: empty host means "no statsd forwarding".
        self.config.metrics.statsd_forward_host = non_empty(value);
    }

    fn consume_statsd_forward_port(&mut self, value: f64) {
        self.config.metrics.statsd_forward_port = f64_to_u16(value);
    }

    fn consume_multi_region_failover_enabled(&mut self, value: bool) {
        // Partial support: ADP supports metric failover only.
        self.config.metrics.multi_region_failover.enabled = value;
    }

    fn consume_multi_region_failover_failover_metrics(&mut self, value: bool) {
        self.config.metrics.multi_region_failover.failover_metrics = value;
    }

    fn consume_multi_region_failover_api_key(&mut self, value: Option<Value>) {
        self.config.metrics.multi_region_failover.api_key = value;
    }

    fn consume_multi_region_failover_dd_url(&mut self, value: Option<Value>) {
        self.config.metrics.multi_region_failover.dd_url = value;
    }

    fn consume_multi_region_failover_site(&mut self, value: Option<Value>) {
        self.config.metrics.multi_region_failover.site = value;
    }

    fn consume_multi_region_failover_metric_allowlist(&mut self, value: Option<Value>) {
        self.config.metrics.multi_region_failover.metric_allowlist = value;
    }

    fn consume_observability_pipelines_worker_metrics_enabled(&mut self, value: bool) {
        self.config.metrics.observability_pipelines_worker_metrics.enabled = value;
    }

    fn consume_observability_pipelines_worker_metrics_url(&mut self, value: String) {
        self.config.metrics.observability_pipelines_worker_metrics.url = value;
    }

    fn consume_vector_metrics_enabled(&mut self, value: bool) {
        self.config.metrics.vector_metrics.enabled = value;
    }

    fn consume_vector_metrics_url(&mut self, value: String) {
        self.config.metrics.vector_metrics.url = value;
    }

    // --- traces / APM -------------------------------------------------------------------------

    fn consume_env(&mut self, value: String) {
        self.config.traces.env = value;
    }

    fn consume_log_payloads(&mut self, value: bool) {
        self.config.traces.log_payloads = value;
    }

    fn consume_apm_config_obfuscation_credit_cards_enabled(&mut self, value: bool) {
        self.config.traces.obfuscation.credit_cards.enabled = value;
    }

    fn consume_apm_config_obfuscation_credit_cards_keep_values(&mut self, value: Vec<String>) {
        self.config.traces.obfuscation.credit_cards.keep_values = value;
    }

    fn consume_apm_config_obfuscation_credit_cards_luhn(&mut self, value: bool) {
        self.config.traces.obfuscation.credit_cards.luhn = value;
    }

    fn consume_apm_config_obfuscation_http_remove_paths_with_digits(&mut self, value: bool) {
        self.config.traces.obfuscation.http.remove_paths_with_digits = value;
    }

    fn consume_apm_config_obfuscation_http_remove_query_string(&mut self, value: bool) {
        self.config.traces.obfuscation.http.remove_query_string = value;
    }

    fn consume_apm_config_obfuscation_elasticsearch_enabled(&mut self, value: bool) {
        self.config.traces.obfuscation.elasticsearch.enabled = value;
    }

    fn consume_apm_config_obfuscation_elasticsearch_keep_values(&mut self, value: Vec<String>) {
        self.config.traces.obfuscation.elasticsearch.keep_values = value;
    }

    fn consume_apm_config_obfuscation_elasticsearch_obfuscate_sql_values(&mut self, value: Vec<String>) {
        self.config.traces.obfuscation.elasticsearch.obfuscate_sql_values = value;
    }

    fn consume_apm_config_obfuscation_mongodb_enabled(&mut self, value: bool) {
        self.config.traces.obfuscation.mongodb.enabled = value;
    }

    fn consume_apm_config_obfuscation_mongodb_keep_values(&mut self, value: Vec<String>) {
        self.config.traces.obfuscation.mongodb.keep_values = value;
    }

    fn consume_apm_config_obfuscation_mongodb_obfuscate_sql_values(&mut self, value: Vec<String>) {
        self.config.traces.obfuscation.mongodb.obfuscate_sql_values = value;
    }

    fn consume_apm_config_obfuscation_opensearch_enabled(&mut self, value: bool) {
        self.config.traces.obfuscation.opensearch.enabled = value;
    }

    fn consume_apm_config_obfuscation_opensearch_keep_values(&mut self, value: Vec<String>) {
        self.config.traces.obfuscation.opensearch.keep_values = value;
    }

    fn consume_apm_config_obfuscation_opensearch_obfuscate_sql_values(&mut self, value: Vec<String>) {
        self.config.traces.obfuscation.opensearch.obfuscate_sql_values = value;
    }

    fn consume_apm_config_obfuscation_memcached_enabled(&mut self, value: bool) {
        self.config.traces.obfuscation.memcached.enabled = value;
    }

    fn consume_apm_config_obfuscation_memcached_keep_command(&mut self, value: bool) {
        self.config.traces.obfuscation.memcached.keep_command = value;
    }

    fn consume_apm_config_obfuscation_redis_enabled(&mut self, value: bool) {
        self.config.traces.obfuscation.redis.enabled = value;
    }

    fn consume_apm_config_obfuscation_redis_remove_all_args(&mut self, value: bool) {
        self.config.traces.obfuscation.redis.remove_all_args = value;
    }

    fn consume_apm_config_obfuscation_valkey_enabled(&mut self, value: bool) {
        self.config.traces.obfuscation.valkey.enabled = value;
    }

    fn consume_apm_config_obfuscation_valkey_remove_all_args(&mut self, value: bool) {
        self.config.traces.obfuscation.valkey.remove_all_args = value;
    }

    // --- OTLP ---------------------------------------------------------------------------------

    fn consume_otlp_config_metrics_enabled(&mut self, value: bool) {
        self.config.otlp.metrics_enabled = value;
    }

    fn consume_otlp_config_logs_enabled(&mut self, value: bool) {
        self.config.otlp.logs_enabled = value;
    }

    fn consume_otlp_config_traces_enabled(&mut self, value: bool) {
        self.config.otlp.traces_enabled = value;
    }

    fn consume_otlp_config_traces_internal_port(&mut self, value: f64) {
        self.config.otlp.traces_internal_port = f64_to_u16(value);
    }

    fn consume_otlp_config_traces_probabilistic_sampler_sampling_percentage(&mut self, value: f64) {
        self.config.otlp.traces_sampling_percentage = value;
    }

    fn consume_otlp_config_receiver_protocols_grpc_endpoint(&mut self, value: String) {
        self.config.otlp.grpc.endpoint = value;
    }

    fn consume_otlp_config_receiver_protocols_grpc_transport(&mut self, value: String) {
        self.config.otlp.grpc.transport = value;
    }

    fn consume_otlp_config_receiver_protocols_grpc_max_recv_msg_size_mib(&mut self, value: f64) {
        self.config.otlp.grpc.max_recv_msg_size_mib = f64_to_u64(value);
    }

    fn consume_otlp_config_receiver_protocols_http_endpoint(&mut self, value: String) {
        self.config.otlp.http.endpoint = value;
    }

    // --- logs ---------------------------------------------------------------------------------

    fn consume_log_level(&mut self, value: String) {
        // Partial support: ADP maps the Agent's level vocabulary onto its own filter.
        // Normalise to lowercase so remote-config delivering "INFO" does not fire a spurious diff.
        self.config.logs.log_level = value.to_lowercase();
    }

    fn consume_log_format_rfc3339(&mut self, value: bool) {
        self.config.logs.log_format_rfc3339 = value;
    }

    fn consume_syslog_rfc(&mut self, value: bool) {
        self.config.logs.syslog_rfc = value;
    }

    fn consume_syslog_uri(&mut self, value: String) {
        self.config.logs.syslog_uri = value;
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// Translate a config whose only change from default is a single key, set via JSON, so each
    /// test exercises the real deserialization + translation path.
    fn translate_with(key: &str, value: serde_json::Value) -> TotalSalukiConfiguration {
        let mut map = serde_json::Map::new();
        map.insert(key.to_string(), value);
        let config: DatadogConfiguration =
            serde_json::from_value(serde_json::Value::Object(map)).expect("config deserializes");
        translate(&config)
    }

    // (a) Smoke test: defaults translate to representative native defaults across subsystems.
    #[test]
    fn default_config_translates_representative_defaults() {
        let out = translate(&DatadogConfiguration::default());

        // cross-cutting
        assert_eq!(out.cross_cutting.cmd_port, 5001);
        assert_eq!(out.cross_cutting.cri_query_timeout_secs, 5);
        // data plane
        assert_eq!(out.data_plane.aggregator_tag_filter_cache_capacity, 100_000);
        // dogstatsd
        assert_eq!(out.dogstatsd.port, 8125);
        assert_eq!(out.dogstatsd.tag_cardinality, "low");
        assert!(
            out.dogstatsd.logging_enabled,
            "dogstatsd_logging_enabled defaults to true"
        );
        assert_eq!(out.dogstatsd.mapper_cache_size, 1000);
        // forwarder
        assert_eq!(out.forwarder.timeout_secs, 20);
        assert_eq!(out.forwarder.http_protocol, "auto");
        assert_eq!(out.forwarder.min_tls_version, TlsVersion::Tls12);
        assert!(!out.forwarder.skip_ssl_validation);
        assert_eq!(out.forwarder.proxy, Default::default());
        // metrics
        assert_eq!(out.metrics.serializer_compressor_kind, "zstd");
        assert_eq!(out.metrics.serializer_zstd_compressor_level, 1);
        assert_eq!(
            out.metrics.histogram_aggregates,
            vec![
                "max".to_string(),
                "median".to_string(),
                "avg".to_string(),
                "count".to_string()
            ]
        );
        assert!(out.metrics.enable_series, "enable_payloads.series defaults to true");
        assert_eq!(out.metrics.statsd_forward_host, None);
        // traces
        assert!(
            out.traces.obfuscation.credit_cards.enabled,
            "credit card obfuscation on by default"
        );
        // otlp
        assert_eq!(out.otlp.grpc.endpoint, "localhost:4317");
        assert_eq!(out.otlp.http.endpoint, "localhost:4318");
        assert!(out.otlp.metrics_enabled, "otlp metrics default enabled");
        assert!(!out.otlp.logs_enabled, "otlp logs default disabled");
        assert_eq!(out.otlp.traces_internal_port, 5003);
        // logs
        assert_eq!(out.logs.log_level, "info");
    }

    // (b) Representative per-subsystem: prove method bodies are not no-ops.

    #[test]
    fn dogstatsd_port_is_set_and_narrowed() {
        let out = translate_with("dogstatsd_port", json!(9000));
        assert_eq!(out.dogstatsd.port, 9000);
    }

    #[test]
    fn dogstatsd_tags_pass_through() {
        let out = translate_with("dogstatsd_tags", json!(["team:adp", "env:test"]));
        assert_eq!(out.dogstatsd.tags, vec!["team:adp".to_string(), "env:test".to_string()]);
    }

    #[test]
    fn forwarder_timeout_is_set() {
        let out = translate_with("forwarder_timeout", json!(45));
        assert_eq!(out.forwarder.timeout_secs, 45);
    }

    #[test]
    fn metrics_serializer_kind_is_set() {
        let out = translate_with("serializer_compressor_kind", json!("zlib"));
        assert_eq!(out.metrics.serializer_compressor_kind, "zlib");
    }

    #[test]
    fn traces_env_is_set() {
        let out = translate_with("env", json!("staging"));
        assert_eq!(out.traces.env, "staging");
    }

    #[test]
    fn otlp_grpc_endpoint_is_set() {
        let config = DatadogConfiguration {
            otlp_config: serde_json::from_value(json!({
                "receiver": {"protocols": {"grpc": {"endpoint": "0.0.0.0:4317"}}}
            }))
            .unwrap(),
            ..Default::default()
        };
        let out = translate(&config);
        assert_eq!(out.otlp.grpc.endpoint, "0.0.0.0:4317");
    }

    #[test]
    fn otlp_traces_sampling_percentage_reaches_native() {
        // The nested probabilistic-sampler percentage must land on the native OTLP slice. This is the
        // value the migrated OTLP source/decoder read instead of deserializing it themselves; the flat
        // env-var form is folded onto this nested path at the translation boundary before translate().
        let config = DatadogConfiguration {
            otlp_config: serde_json::from_value(json!({
                "traces": {"probabilistic_sampler": {"sampling_percentage": 42.5}}
            }))
            .unwrap(),
            ..Default::default()
        };
        let out = translate(&config);
        assert_eq!(out.otlp.traces_sampling_percentage, 42.5);
    }

    #[test]
    fn logs_level_is_set() {
        let out = translate_with("log_level", json!("debug"));
        assert_eq!(out.logs.log_level, "debug");
    }

    // (c) Tricky cases.

    #[test]
    fn min_tls_version_clamps_old_versions_to_1_2() {
        // 1.0 and 1.1 are accepted for compatibility but clamped up to 1.2 (rustls limitation).
        for v in ["tlsv1.0", "TLSv1.0", "tlsv1.1"] {
            let out = translate_with("min_tls_version", json!(v));
            assert_eq!(
                out.forwarder.min_tls_version,
                TlsVersion::Tls12,
                "{v} should clamp to 1.2"
            );
        }
    }

    #[test]
    fn min_tls_version_passes_through_1_2_and_1_3() {
        assert_eq!(
            translate_with("min_tls_version", json!("tlsv1.2"))
                .forwarder
                .min_tls_version,
            TlsVersion::Tls12
        );
        assert_eq!(
            translate_with("min_tls_version", json!("TLSv1.3"))
                .forwarder
                .min_tls_version,
            TlsVersion::Tls13
        );
    }

    #[test]
    fn min_tls_version_unknown_falls_back_to_1_2() {
        let out = translate_with("min_tls_version", json!("bogus"));
        assert_eq!(out.forwarder.min_tls_version, TlsVersion::Tls12);
    }

    #[test]
    fn additional_endpoints_map_passes_through() {
        let out = translate_with(
            "additional_endpoints",
            json!({"https://extra.example.com": ["key-a", "key-b"]}),
        );
        let endpoints = &out.forwarder.additional_endpoints;
        assert_eq!(endpoints.len(), 1);
        assert_eq!(
            endpoints.get("https://extra.example.com"),
            Some(&vec!["key-a".to_string(), "key-b".to_string()])
        );
    }

    #[test]
    fn proxy_three_fields_assemble_into_one_struct() {
        let config = DatadogConfiguration {
            proxy: serde_json::from_value(json!({
                "http": "http://proxy:3128",
                "https": "https://proxy:3129",
                "no_proxy": ["localhost", "169.254.169.254"]
            }))
            .unwrap(),
            ..Default::default()
        };
        let out = translate(&config);
        assert_eq!(out.forwarder.proxy.http.as_deref(), Some("http://proxy:3128"));
        assert_eq!(out.forwarder.proxy.https.as_deref(), Some("https://proxy:3129"));
        assert_eq!(
            out.forwarder.proxy.no_proxy,
            vec!["localhost".to_string(), "169.254.169.254".to_string()]
        );
    }

    #[test]
    fn proxy_empty_strings_become_none() {
        // The schema uses "" as the unset sentinel for proxy URLs; the native model uses None.
        let out = translate(&DatadogConfiguration::default());
        assert_eq!(out.forwarder.proxy.http, None);
        assert_eq!(out.forwarder.proxy.https, None);
    }

    #[test]
    fn f64_port_narrows_and_saturates() {
        // Out-of-range schema numbers saturate into the u16 range rather than wrapping.
        assert_eq!(translate_with("dogstatsd_port", json!(70000)).dogstatsd.port, u16::MAX);
        assert_eq!(translate_with("dogstatsd_port", json!(-1)).dogstatsd.port, 0);
    }

    #[test]
    fn opaque_serde_json_value_carried_raw() {
        // dd_url is an opaque schema value (Option<serde_json::Value>); it is stored verbatim.
        let out = translate_with("dd_url", json!("https://custom.intake.example.com"));
        assert_eq!(
            out.forwarder.dd_url,
            Some(serde_json::Value::String(
                "https://custom.intake.example.com".to_string()
            ))
        );
        // Absent => None.
        assert_eq!(translate(&DatadogConfiguration::default()).forwarder.dd_url, None);
    }

    #[test]
    fn dogstatsd_socket_empty_string_is_none() {
        // An explicit empty socket path disables UDS, same as absence.
        let out = translate_with("dogstatsd_socket", json!(""));
        assert_eq!(out.dogstatsd.socket, None);
        let out = translate_with("dogstatsd_socket", json!("/var/run/dsd.sock"));
        assert_eq!(out.dogstatsd.socket.as_deref(), Some("/var/run/dsd.sock"));
    }

    #[test]
    fn mapper_cache_size_zero_passes_through() {
        // Partial-support divergence: 0 is carried through (ADP disables only the cache).
        let out = translate_with("dogstatsd_mapper_cache_size", json!(0));
        assert_eq!(out.dogstatsd.mapper_cache_size, 0);
    }

    // (d) f64 narrowing edge cases beyond the u16 port path already covered above.

    #[test]
    fn f64_to_i32_negative_and_saturates() {
        // The zstd compressor level is signed: negatives pass through, out-of-range values saturate.
        let neg = translate_with("serializer_zstd_compressor_level", json!(-5));
        assert_eq!(neg.metrics.serializer_zstd_compressor_level, -5);

        let over = translate_with("serializer_zstd_compressor_level", json!(5_000_000_000.0_f64));
        assert_eq!(over.metrics.serializer_zstd_compressor_level, i32::MAX);

        let under = translate_with("serializer_zstd_compressor_level", json!(-5_000_000_000.0_f64));
        assert_eq!(under.metrics.serializer_zstd_compressor_level, i32::MIN);
    }

    #[test]
    fn f64_to_u64_saturates_large_and_negative() {
        // forwarder_timeout is an unsigned count: huge values clamp to u64::MAX, negatives to 0.
        let over = translate_with("forwarder_timeout", json!(2.0e19_f64));
        assert_eq!(over.forwarder.timeout_secs, u64::MAX);

        let neg = translate_with("forwarder_timeout", json!(-1));
        assert_eq!(neg.forwarder.timeout_secs, 0);
    }

    #[test]
    fn statsd_forward_host_and_port_set() {
        // A non-empty host maps to Some(host)...
        let host = translate_with("statsd_forward_host", json!("relay.example.com"));
        assert_eq!(host.metrics.statsd_forward_host.as_deref(), Some("relay.example.com"));
        // ...and the forward port narrows from the schema's f64.
        let port = translate_with("statsd_forward_port", json!(8126));
        assert_eq!(port.metrics.statsd_forward_port, 8126);
    }

    #[test]
    fn mrf_metric_allowlist_opaque_value_round_trips() {
        // An MRF override is an opaque Option<serde_json::Value> nested under an Option<Section>;
        // it must reach its native field verbatim.
        let config = DatadogConfiguration {
            multi_region_failover: serde_json::from_value(json!({
                "metric_allowlist": ["system.cpu.user", "system.mem.used"]
            }))
            .unwrap(),
            ..Default::default()
        };
        let out = translate(&config);
        assert_eq!(
            out.metrics.multi_region_failover.metric_allowlist,
            Some(json!(["system.cpu.user", "system.mem.used"]))
        );
    }

    // (e) min_tls_version tolerates surrounding whitespace (otherwise silently downgraded to 1.2).

    #[test]
    fn min_tls_version_trims_whitespace_and_case() {
        let out = translate_with("min_tls_version", json!("  TLSv1.3 "));
        assert_eq!(out.forwarder.min_tls_version, TlsVersion::Tls13);
    }
}
