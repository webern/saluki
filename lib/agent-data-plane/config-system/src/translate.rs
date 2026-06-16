//! Witness-driven translation from the Datadog source model into [`SalukiConfiguration`].
//!
//! [`Translator`] implements the generated [`DatadogConfigConsumer`] trait, so the compiler forces
//! every supported Datadog key to have an explicit destination here. Most keys route into a native
//! `saluki-component-config` accumulator; a handful that ADP does not yet model are acknowledged
//! with an explicit no-op body (which the witness still requires us to write). Saluki-private
//! supplemental configuration is folded in alongside, so no component ever reads a private knob from
//! a raw map.

use agent_data_plane_config::{
    ChecksConfigs, DataPlaneConfig, DogStatsDConfigs, EventsConfigs, ForwarderConfigs, LogsConfigs, MetricsConfigs,
    OtlpConfigs, PipelineGate, RuntimeLoggingConfig, SalukiConfiguration, SalukiPrivateConfiguration,
    ServiceChecksConfigs, TracesConfigs,
};
use datadog_agent_config::{drive, DatadogConfigConsumer, DatadogConfiguration};
use saluki_component_config::common::TlsMinimumVersion;
use saluki_component_config::common::{CompressionConfig, CompressionKind, EndpointConfig, TlsClientConfig};
use saluki_component_config::{
    AggregateConfig, ApmStatsEncoderConfig, DatadogEventsEncoderConfig, DatadogForwarderConfig,
    DatadogLogsEncoderConfig, DatadogMetricsEncoderConfig, DatadogServiceChecksEncoderConfig,
    DatadogTracesEncoderConfig, DogStatsDConfig, DogStatsDDebugLogConfig, DogStatsDMapperConfig,
    MetricsEnrichmentConfig, MultiRegionFailoverConfig, OtlpConfig, OtlpProxyConfig, PrefixFilterConfig,
    TagFilterlistConfig, TraceObfuscationConfig, TraceSamplerConfig, TracesEnrichmentConfig,
};
use saluki_io::net::ListenAddress;
use std::path::PathBuf;
use std::time::Duration;
use stringtheory::MetaString;

/// The set of pipeline enable/disable gates ADP control keys, resolved outside the witnessed Datadog
/// schema (these `data_plane.*` gates are ADP control inputs, not part of the supported schema
/// surface). The configuration system supplies them when constructing a translator.
#[derive(Clone, Copy, Debug, Default)]
pub struct PipelineGates {
    /// Whether the data plane is enabled at all.
    pub enabled: bool,
    /// Whether the DogStatsD pipeline is enabled.
    pub dogstatsd_enabled: bool,
    /// Whether the checks pipeline is enabled.
    pub checks_enabled: bool,
    /// Whether the OTLP pipeline is enabled.
    pub otlp_enabled: bool,
}

/// Accumulates a [`SalukiConfiguration`] while a [`DatadogConfiguration`] is driven over it.
pub struct Translator {
    gates: PipelineGates,

    logging: RuntimeLoggingConfig,
    forwarder: DatadogForwarderConfig,
    metrics_enrichment: MetricsEnrichmentConfig,
    metrics_encoder: DatadogMetricsEncoderConfig,
    logs_encoder: DatadogLogsEncoderConfig,
    events_encoder: DatadogEventsEncoderConfig,
    service_checks_encoder: DatadogServiceChecksEncoderConfig,
    traces_enrichment: TracesEnrichmentConfig,
    trace_sampler: TraceSamplerConfig,
    trace_obfuscation: TraceObfuscationConfig,
    apm_stats_encoder: ApmStatsEncoderConfig,
    traces_encoder: DatadogTracesEncoderConfig,
    dsd_source: DogStatsDConfig,
    dsd_mapper: DogStatsDMapperConfig,
    dsd_prefix_filter: PrefixFilterConfig,
    dsd_tag_filterlist: TagFilterlistConfig,
    dsd_aggregate: AggregateConfig,
    dsd_debug_log: DogStatsDDebugLogConfig,
    otlp: OtlpConfig,

    // Shared compression, applied to every encoder at `finish`.
    compression: CompressionConfig,

    // Workload tuning knobs carried through to `SalukiConfiguration` for the environment provider.
    workload: agent_data_plane_config::WorkloadPrivateConfig,

    // Staged endpoint inputs assembled into the forwarder endpoints at `finish`.
    api_key: Option<String>,
    dd_url: Option<String>,
    site: Option<String>,
    additional_endpoints: serde_json::Value,

    // Staged MRF inputs.
    mrf_enabled: bool,
    mrf_failover_metrics: bool,
    mrf_metric_allowlist: Vec<MetaString>,
    mrf_api_key: Option<String>,
    mrf_dd_url: Option<String>,
    mrf_site: Option<String>,

    // Staged listen addresses.
    api_listen_address: Option<String>,
    secure_api_listen_address: Option<String>,

    // Staged TLS inputs.
    skip_ssl_validation: bool,
    min_tls_version: TlsMinimumVersion,
    ssl_key_log_file: Option<String>,

    // Staged OTLP proxy inputs.
    otlp_proxy_metrics: bool,
    otlp_proxy_logs: bool,
    otlp_proxy_traces: bool,
    otlp_proxy_grpc_endpoint: Option<String>,
    otlp_traces_internal_port: u16,

    // Staged DogStatsD debug-log gate.
    dsd_logging_enabled: bool,
}

impl Translator {
    /// Creates a translator seeded with defaults plus the given Saluki-private supplemental config.
    pub fn new(private: &SalukiPrivateConfiguration, gates: PipelineGates) -> Self {
        // Fold the private knobs the Datadog language cannot express into the native defaults.
        let otlp = OtlpConfig {
            context_string_interner_bytes: private.otlp.context_string_interner_bytes,
            cached_contexts_limit: private.otlp.cached_contexts_limit,
            cached_tagsets_limit: private.otlp.cached_tagsets_limit,
            allow_context_heap_allocations: private.otlp.allow_context_heap_allocations,
            ..Default::default()
        };

        let dsd_mapper = DogStatsDMapperConfig {
            context_string_interner_bytes: private.dogstatsd.mapper_string_interner_bytes,
            cache_size: private.dogstatsd.mapper_cache_size,
            ..Default::default()
        };

        Self {
            gates,
            logging: RuntimeLoggingConfig::default(),
            forwarder: DatadogForwarderConfig::default(),
            metrics_enrichment: MetricsEnrichmentConfig::default(),
            metrics_encoder: DatadogMetricsEncoderConfig::default(),
            logs_encoder: DatadogLogsEncoderConfig::default(),
            events_encoder: DatadogEventsEncoderConfig::default(),
            service_checks_encoder: DatadogServiceChecksEncoderConfig::default(),
            traces_enrichment: TracesEnrichmentConfig::default(),
            trace_sampler: TraceSamplerConfig::default(),
            trace_obfuscation: TraceObfuscationConfig::default(),
            apm_stats_encoder: ApmStatsEncoderConfig::default(),
            traces_encoder: DatadogTracesEncoderConfig::default(),
            dsd_source: DogStatsDConfig::default(),
            dsd_mapper,
            dsd_prefix_filter: PrefixFilterConfig::default(),
            dsd_tag_filterlist: TagFilterlistConfig::default(),
            dsd_aggregate: AggregateConfig::default(),
            dsd_debug_log: DogStatsDDebugLogConfig {
                metrics_stats_enabled: false,
                logging_enabled: false,
                log_file: PathBuf::new(),
                log_file_max_size: bytesize::ByteSize::mib(10),
                log_file_max_rolls: 1,
            },
            otlp,
            compression: private.compression,
            workload: private.workload.clone(),
            api_key: None,
            dd_url: None,
            site: None,
            additional_endpoints: serde_json::Value::Null,
            mrf_enabled: false,
            mrf_failover_metrics: false,
            mrf_metric_allowlist: Vec::new(),
            mrf_api_key: None,
            mrf_dd_url: None,
            mrf_site: None,
            api_listen_address: None,
            secure_api_listen_address: None,
            skip_ssl_validation: false,
            min_tls_version: TlsMinimumVersion::Tls1_2,
            ssl_key_log_file: None,
            otlp_proxy_metrics: true,
            otlp_proxy_logs: true,
            otlp_proxy_traces: true,
            otlp_proxy_grpc_endpoint: None,
            otlp_traces_internal_port: 0,
            dsd_logging_enabled: false,
        }
    }

    /// Assemble the accumulated state into a complete [`SalukiConfiguration`].
    pub fn finish(mut self) -> SalukiConfiguration {
        // Apply shared compression to every encoder.
        self.metrics_encoder.compression = self.compression;
        self.logs_encoder.compression = self.compression;
        self.events_encoder.compression = self.compression;
        self.service_checks_encoder.compression = self.compression;
        self.traces_encoder.compression = self.compression;

        // Apply staged TLS to the forwarder.
        self.forwarder.tls = TlsClientConfig {
            skip_ssl_validation: self.skip_ssl_validation,
            min_tls_version: self.min_tls_version,
            ssl_key_log_file: self.ssl_key_log_file.as_deref().map(MetaString::from),
        };

        // Assemble forwarder endpoints from the staged primary endpoint plus additional endpoints.
        self.forwarder.endpoints = build_endpoints(
            self.api_key.as_deref(),
            self.dd_url.as_deref(),
            self.site.as_deref(),
            &self.additional_endpoints,
        );

        // OTLP proxy: present only when a proxy endpoint was configured.
        self.otlp.proxy = self
            .otlp_proxy_grpc_endpoint
            .as_deref()
            .map(|endpoint| OtlpProxyConfig {
                core_agent_otlp_grpc_endpoint: MetaString::from(endpoint),
                proxy_metrics: self.otlp_proxy_metrics,
                proxy_logs: self.otlp_proxy_logs,
                proxy_traces: self.otlp_proxy_traces,
                core_agent_traces_internal_port: self.otlp_traces_internal_port,
            });
        let otlp_proxy_enabled = self.otlp.proxy.is_some();

        // MRF: present only when enabled, with its own endpoint/API key.
        let multi_region_failover = if self.mrf_enabled {
            let mut mrf_forwarder = self.forwarder.clone();
            mrf_forwarder.endpoints = build_endpoints(
                self.mrf_api_key.as_deref(),
                self.mrf_dd_url.as_deref(),
                self.mrf_site.as_deref(),
                &serde_json::Value::Null,
            );
            Some(MultiRegionFailoverConfig {
                failover_metrics: self.mrf_failover_metrics,
                metric_allowlist: self.mrf_metric_allowlist.clone(),
                forwarder: mrf_forwarder,
            })
        } else {
            None
        };

        let data_plane = DataPlaneConfig {
            enabled: self.gates.enabled,
            api_listen_address: parse_listen_address(self.api_listen_address.as_deref(), 5100),
            secure_api_listen_address: parse_listen_address(self.secure_api_listen_address.as_deref(), 5101),
            dogstatsd: PipelineGate::new(self.gates.dogstatsd_enabled),
            checks: PipelineGate::new(self.gates.checks_enabled),
            otlp: PipelineGate::new(self.gates.otlp_enabled),
            otlp_proxy_enabled,
            otlp_proxy_traces: self.otlp_proxy_traces,
        };

        let debug_log = if self.dsd_debug_log.metrics_stats_enabled || self.dsd_logging_enabled {
            self.dsd_debug_log.logging_enabled = self.dsd_logging_enabled;
            Some(self.dsd_debug_log)
        } else {
            None
        };

        SalukiConfiguration {
            logging: self.logging,
            data_plane,
            // Memory bounds are an ADP control input outside the witnessed schema; populated by the
            // configuration system after translation.
            memory: agent_data_plane_config::MemoryConfig::default(),
            forwarder: ForwarderConfigs {
                datadog: self.forwarder,
            },
            metrics: MetricsConfigs {
                enrichment: self.metrics_enrichment,
                datadog_encoder: self.metrics_encoder,
                multi_region_failover,
            },
            logs: LogsConfigs {
                datadog_encoder: self.logs_encoder,
            },
            events: EventsConfigs {
                datadog_encoder: self.events_encoder,
            },
            service_checks: ServiceChecksConfigs {
                datadog_encoder: self.service_checks_encoder,
            },
            traces: TracesConfigs {
                enrichment: self.traces_enrichment,
                sampler: self.trace_sampler,
                obfuscation: self.trace_obfuscation,
                apm_stats_encoder: self.apm_stats_encoder,
                datadog_encoder: self.traces_encoder,
            },
            checks: ChecksConfigs {
                ipc: saluki_component_config::ChecksConfig {
                    grpc_endpoint: ListenAddress::any_tcp(5004),
                },
            },
            dogstatsd: DogStatsDConfigs {
                source: self.dsd_source,
                mapper: self.dsd_mapper,
                prefix_filter: self.dsd_prefix_filter,
                tag_filterlist: self.dsd_tag_filterlist,
                aggregate: self.dsd_aggregate,
                debug_log,
            },
            otlp: OtlpConfigs { config: self.otlp },
            workload: self.workload,
        }
    }
}

/// Translate a Datadog source configuration plus Saluki-private config into the ADP-native model.
pub fn translate_datadog(
    config: &DatadogConfiguration, private: &SalukiPrivateConfiguration, gates: PipelineGates,
) -> SalukiConfiguration {
    let mut translator = Translator::new(private, gates);
    drive(config, &mut translator);
    translator.finish()
}

fn build_endpoints(
    api_key: Option<&str>, dd_url: Option<&str>, site: Option<&str>, _additional: &serde_json::Value,
) -> Vec<EndpointConfig> {
    let api_key = match api_key {
        Some(key) if !key.is_empty() => key,
        _ => return Vec::new(),
    };
    let url = resolve_intake_url(dd_url, site);
    vec![EndpointConfig::new(url, api_key)]
}

fn resolve_intake_url(dd_url: Option<&str>, site: Option<&str>) -> String {
    match dd_url {
        Some(url) if !url.is_empty() => url.to_string(),
        _ => match site {
            Some(site) if !site.is_empty() => format!("https://app.{site}"),
            _ => "https://app.datadoghq.com".to_string(),
        },
    }
}

fn parse_listen_address(raw: Option<&str>, default_port: u16) -> ListenAddress {
    match raw {
        Some(value) if !value.is_empty() => {
            ListenAddress::try_from(value).unwrap_or_else(|_| ListenAddress::any_tcp(default_port))
        }
        _ => ListenAddress::any_tcp(default_port),
    }
}

fn secs(value: i64) -> Duration {
    Duration::from_secs(value.max(0) as u64)
}

fn metastrings(values: Vec<String>) -> Vec<MetaString> {
    values.into_iter().map(MetaString::from).collect()
}

impl DatadogConfigConsumer for Translator {
    // ----- Endpoints / forwarder authentication -----
    fn consume_api_key(&mut self, value: Option<String>) {
        self.api_key = value;
    }
    fn consume_dd_url(&mut self, value: Option<String>) {
        self.dd_url = value;
    }
    fn consume_site(&mut self, value: Option<String>) {
        self.site = value;
    }
    fn consume_additional_endpoints(&mut self, value: serde_json::Value) {
        self.additional_endpoints = value;
    }
    fn consume_allow_arbitrary_tags(&mut self, value: bool) {
        self.forwarder.allow_arbitrary_tags = value;
    }

    // ----- Forwarder tuning -----
    fn consume_forwarder_max_concurrent_requests(&mut self, value: i64) {
        self.forwarder.endpoint_concurrency = value.max(1) as usize;
    }
    fn consume_forwarder_num_workers(&mut self, value: i64) {
        self.forwarder.endpoint_concurrency_multiplier = value.max(1) as usize;
    }
    fn consume_forwarder_timeout(&mut self, value: i64) {
        self.forwarder.request_timeout = secs(value);
    }
    fn consume_forwarder_high_prio_buffer_size(&mut self, value: i64) {
        self.forwarder.endpoint_buffer_size = value.max(0) as usize;
    }
    fn consume_forwarder_connection_reset_interval(&mut self, value: i64) {
        self.forwarder.connection_reset_interval = (value > 0).then(|| secs(value));
    }
    fn consume_forwarder_backoff_base(&mut self, value: i64) {
        self.forwarder.retry.base_backoff = secs(value);
    }
    fn consume_forwarder_backoff_max(&mut self, value: i64) {
        self.forwarder.retry.max_backoff = secs(value);
    }
    fn consume_forwarder_storage_max_size_in_bytes(&mut self, value: i64) {
        self.forwarder.retry.disk_persistence_enabled = value > 0;
    }
    // Forwarder knobs ADP does not yet model natively.
    fn consume_forwarder_apikey_validation_interval(&mut self, _value: i64) {}
    fn consume_forwarder_backoff_factor(&mut self, _value: i64) {}
    fn consume_forwarder_http_protocol(&mut self, _value: Option<String>) {}
    fn consume_forwarder_outdated_file_in_days(&mut self, _value: i64) {}
    fn consume_forwarder_recovery_interval(&mut self, _value: i64) {}
    fn consume_forwarder_recovery_reset(&mut self, _value: bool) {}
    fn consume_forwarder_retry_queue_capacity_time_interval_sec(&mut self, _value: i64) {}
    fn consume_forwarder_retry_queue_max_size(&mut self, _value: i64) {}
    fn consume_forwarder_retry_queue_payloads_max_size(&mut self, _value: i64) {}
    fn consume_forwarder_storage_max_disk_ratio(&mut self, _value: f64) {}
    fn consume_forwarder_storage_path(&mut self, _value: Option<String>) {}

    // ----- TLS / proxy -----
    fn consume_skip_ssl_validation(&mut self, value: bool) {
        self.skip_ssl_validation = value;
    }
    fn consume_min_tls_version(&mut self, value: Option<String>) {
        // Clamp unsupported source values down to TLS 1.2 (rustls supports 1.2 and 1.3 only).
        self.min_tls_version = match value.as_deref() {
            Some("tlsv1.3") | Some("TLSv1.3") => TlsMinimumVersion::Tls1_3,
            _ => TlsMinimumVersion::Tls1_2,
        };
    }
    fn consume_sslkeylogfile(&mut self, value: Option<String>) {
        self.ssl_key_log_file = value.filter(|s| !s.is_empty());
    }
    fn consume_proxy_http(&mut self, _value: Option<String>) {}
    fn consume_proxy_https(&mut self, _value: Option<String>) {}
    fn consume_proxy_no_proxy(&mut self, _value: Vec<String>) {}
    fn consume_no_proxy_nonexact_match(&mut self, _value: bool) {}
    fn consume_use_proxy_for_cloud_metadata(&mut self, _value: bool) {}

    // ----- Logging (runtime) -----
    fn consume_log_level(&mut self, value: Option<String>) {
        self.logging.log_level = value;
    }
    fn consume_log_format_rfc3339(&mut self, value: bool) {
        self.logging.log_format_rfc3339 = value;
    }
    fn consume_syslog_rfc(&mut self, value: bool) {
        self.logging.syslog_rfc = value;
    }
    fn consume_syslog_uri(&mut self, value: Option<String>) {
        if let Some(uri) = value {
            self.logging.syslog_uri = uri;
        }
    }
    fn consume_data_plane_log_file(&mut self, value: Option<String>) {
        if let Some(path) = value {
            self.logging.log_file = path;
        }
    }

    // ----- Metrics encoder (serializer.*) -----
    fn consume_serializer_max_payload_size(&mut self, value: i64) {
        self.metrics_encoder.max_payload_size = value.max(0) as usize;
        self.events_encoder.max_payload_size = value.max(0) as usize;
        self.service_checks_encoder.max_payload_size = value.max(0) as usize;
    }
    fn consume_serializer_max_uncompressed_payload_size(&mut self, value: i64) {
        self.metrics_encoder.max_uncompressed_payload_size = value.max(0) as usize;
        self.events_encoder.max_uncompressed_payload_size = value.max(0) as usize;
        self.service_checks_encoder.max_uncompressed_payload_size = value.max(0) as usize;
    }
    fn consume_serializer_max_series_payload_size(&mut self, value: i64) {
        self.metrics_encoder.max_series_payload_size = value.max(0) as usize;
    }
    fn consume_serializer_max_series_uncompressed_payload_size(&mut self, value: i64) {
        self.metrics_encoder.max_series_uncompressed_payload_size = value.max(0) as usize;
    }
    fn consume_serializer_max_series_points_per_payload(&mut self, value: i64) {
        self.metrics_encoder.max_series_points_per_payload = value.max(0) as usize;
    }
    fn consume_serializer_compressor_kind(&mut self, value: Option<String>) {
        self.compression.kind = match value.as_deref() {
            Some("zlib") | Some("deflate") => CompressionKind::Zlib,
            _ => CompressionKind::Zstd,
        };
    }
    fn consume_serializer_zstd_compressor_level(&mut self, value: i64) {
        self.compression.zstd_level = value as i32;
    }
    fn consume_use_v2_api_series(&mut self, value: bool) {
        self.metrics_encoder.use_v2_api_series = value;
    }
    fn consume_log_payloads(&mut self, value: bool) {
        self.events_encoder.log_payloads = value;
        self.service_checks_encoder.log_payloads = value;
        self.metrics_encoder.log_payloads = value;
    }

    // ----- Environment / traces -----
    fn consume_env(&mut self, value: Option<String>) {
        let env = MetaString::from(value.unwrap_or_default());
        self.traces_enrichment.default_env = env.clone();
        self.apm_stats_encoder.default_env = env.clone();
        self.traces_encoder.default_env = env;
    }
    fn consume_otlp_config_traces_probabilistic_sampler_sampling_percentage(&mut self, value: f64) {
        self.trace_sampler.otlp_sampling_rate = (value / 100.0).clamp(0.0, 1.0);
    }
    fn consume_apm_config_obfuscation_credit_cards_enabled(&mut self, value: bool) {
        self.trace_obfuscation.scrub_credit_cards = value;
    }
    // Obfuscation knobs not yet modeled in the native obfuscation summary.
    fn consume_apm_config_obfuscation_credit_cards_keep_values(&mut self, _value: Vec<String>) {}
    fn consume_apm_config_obfuscation_credit_cards_luhn(&mut self, _value: bool) {}
    fn consume_apm_config_obfuscation_elasticsearch_enabled(&mut self, _value: bool) {}
    fn consume_apm_config_obfuscation_elasticsearch_keep_values(&mut self, _value: Vec<String>) {}
    fn consume_apm_config_obfuscation_elasticsearch_obfuscate_sql_values(&mut self, _value: Vec<String>) {}
    fn consume_apm_config_obfuscation_http_remove_paths_with_digits(&mut self, _value: bool) {}
    fn consume_apm_config_obfuscation_http_remove_query_string(&mut self, _value: bool) {}
    fn consume_apm_config_obfuscation_memcached_enabled(&mut self, _value: bool) {}
    fn consume_apm_config_obfuscation_memcached_keep_command(&mut self, _value: bool) {}
    fn consume_apm_config_obfuscation_mongodb_enabled(&mut self, _value: bool) {}
    fn consume_apm_config_obfuscation_mongodb_keep_values(&mut self, _value: Vec<String>) {}
    fn consume_apm_config_obfuscation_mongodb_obfuscate_sql_values(&mut self, _value: Vec<String>) {}
    fn consume_apm_config_obfuscation_opensearch_enabled(&mut self, _value: bool) {}
    fn consume_apm_config_obfuscation_opensearch_keep_values(&mut self, _value: Vec<String>) {}
    fn consume_apm_config_obfuscation_opensearch_obfuscate_sql_values(&mut self, _value: Vec<String>) {}
    fn consume_apm_config_obfuscation_redis_enabled(&mut self, _value: bool) {}
    fn consume_apm_config_obfuscation_redis_remove_all_args(&mut self, _value: bool) {}
    fn consume_apm_config_obfuscation_valkey_enabled(&mut self, _value: bool) {}
    fn consume_apm_config_obfuscation_valkey_remove_all_args(&mut self, _value: bool) {}

    // ----- Multi-region failover -----
    fn consume_multi_region_failover_enabled(&mut self, value: bool) {
        self.mrf_enabled = value;
    }
    fn consume_multi_region_failover_api_key(&mut self, value: Option<String>) {
        self.mrf_api_key = value;
    }
    fn consume_multi_region_failover_dd_url(&mut self, value: Option<String>) {
        self.mrf_dd_url = value;
    }
    fn consume_multi_region_failover_site(&mut self, value: Option<String>) {
        self.mrf_site = value;
    }
    fn consume_multi_region_failover_failover_metrics(&mut self, value: bool) {
        self.mrf_failover_metrics = value;
    }
    fn consume_multi_region_failover_metric_allowlist(&mut self, value: Vec<String>) {
        self.mrf_metric_allowlist = metastrings(value);
    }

    // ----- DogStatsD source -----
    fn consume_dogstatsd_port(&mut self, value: i64) {
        self.dsd_source.port = value.clamp(0, u16::MAX as i64) as u16;
    }
    fn consume_dogstatsd_buffer_size(&mut self, value: i64) {
        self.dsd_source.buffer_size = value.max(0) as usize;
    }
    fn consume_dogstatsd_so_rcvbuf(&mut self, value: i64) {
        self.dsd_source.socket_receive_buffer_size = (value > 0).then_some(value as usize);
    }
    fn consume_dogstatsd_socket(&mut self, value: Option<String>) {
        self.dsd_source.socket_path = value.filter(|s| !s.is_empty()).map(MetaString::from);
    }
    fn consume_dogstatsd_stream_socket(&mut self, value: Option<String>) {
        self.dsd_source.socket_stream_path = value.filter(|s| !s.is_empty()).map(MetaString::from);
    }
    fn consume_dogstatsd_non_local_traffic(&mut self, value: bool) {
        self.dsd_source.non_local_traffic = value;
    }
    fn consume_dogstatsd_origin_detection(&mut self, value: bool) {
        self.dsd_source.origin_detection_enabled = value;
        self.metrics_enrichment.origin_detection_enabled = value;
    }
    fn consume_enable_payloads_series(&mut self, value: bool) {
        self.dsd_source.enabled_payloads.series = value;
    }
    fn consume_enable_payloads_sketches(&mut self, value: bool) {
        self.dsd_source.enabled_payloads.sketches = value;
    }
    fn consume_enable_payloads_events(&mut self, value: bool) {
        self.dsd_source.enabled_payloads.events = value;
    }
    fn consume_enable_payloads_service_checks(&mut self, value: bool) {
        self.dsd_source.enabled_payloads.service_checks = value;
    }

    // ----- DogStatsD aggregation -----
    fn consume_dogstatsd_context_expiry_seconds(&mut self, value: i64) {
        self.dsd_aggregate.counter_expiry = (value > 0).then(|| secs(value));
    }
    fn consume_dogstatsd_flush_incomplete_buckets(&mut self, value: bool) {
        self.dsd_aggregate.flush_open_windows = value;
    }
    fn consume_dogstatsd_no_aggregation_pipeline(&mut self, value: bool) {
        self.dsd_aggregate.passthrough_timestamped_metrics = value;
    }

    // ----- DogStatsD histograms -----
    fn consume_histogram_aggregates(&mut self, value: Vec<String>) {
        if !value.is_empty() {
            self.dsd_aggregate.histogram.statistics = metastrings(value);
        }
    }
    fn consume_histogram_copy_to_distribution(&mut self, value: bool) {
        self.dsd_aggregate.histogram.copy_to_distribution = value;
    }
    fn consume_histogram_copy_to_distribution_prefix(&mut self, value: Option<String>) {
        if let Some(prefix) = value {
            self.dsd_aggregate.histogram.copy_to_distribution_prefix = MetaString::from(prefix);
        }
    }

    // ----- DogStatsD mapper -----
    fn consume_dogstatsd_mapper_cache_size(&mut self, value: i64) {
        self.dsd_mapper.cache_size = value.max(0) as usize;
    }
    fn consume_dogstatsd_mapper_profiles(&mut self, _value: serde_json::Value) {
        // Mapper profiles are summarized in the native model for the spike.
    }
    fn consume_dogstatsd_string_interner_size(&mut self, _value: i64) {}

    // ----- DogStatsD prefix/blocklist filters -----
    fn consume_metric_filterlist(&mut self, value: Vec<String>) {
        self.dsd_prefix_filter.metric_filterlist = metastrings(value);
    }
    fn consume_metric_filterlist_match_prefix(&mut self, value: bool) {
        self.dsd_prefix_filter.metric_filterlist_match_prefix = value;
    }
    fn consume_statsd_metric_blocklist(&mut self, value: Vec<String>) {
        self.dsd_prefix_filter.metric_blocklist = metastrings(value);
    }
    fn consume_statsd_metric_blocklist_match_prefix(&mut self, value: bool) {
        self.dsd_prefix_filter.metric_blocklist_match_prefix = value;
    }

    // ----- DogStatsD debug log -----
    fn consume_dogstatsd_metrics_stats_enable(&mut self, value: bool) {
        self.dsd_debug_log.metrics_stats_enabled = value;
    }
    fn consume_dogstatsd_logging_enabled(&mut self, value: bool) {
        self.dsd_logging_enabled = value;
    }
    fn consume_dogstatsd_log_file(&mut self, value: Option<String>) {
        if let Some(path) = value {
            self.dsd_debug_log.log_file = PathBuf::from(path);
        }
    }
    fn consume_dogstatsd_log_file_max_rolls(&mut self, value: i64) {
        self.dsd_debug_log.log_file_max_rolls = value.max(0) as usize;
    }
    fn consume_dogstatsd_log_file_max_size(&mut self, _value: Option<String>) {}

    // DogStatsD knobs not yet modeled natively.
    fn consume_dogstatsd_capture_depth(&mut self, _value: i64) {}
    fn consume_dogstatsd_capture_path(&mut self, _value: Option<String>) {}
    fn consume_dogstatsd_entity_id_precedence(&mut self, _value: bool) {}
    fn consume_dogstatsd_eol_required(&mut self, _value: Vec<String>) {}
    fn consume_dogstatsd_origin_detection_client(&mut self, _value: bool) {}
    fn consume_dogstatsd_origin_optout_enabled(&mut self, _value: bool) {}
    fn consume_dogstatsd_stream_log_too_big(&mut self, _value: bool) {}
    fn consume_dogstatsd_tag_cardinality(&mut self, _value: Option<String>) {}
    fn consume_dogstatsd_tags(&mut self, _value: Vec<String>) {}
    fn consume_statsd_forward_host(&mut self, _value: Option<String>) {}
    fn consume_statsd_forward_port(&mut self, _value: i64) {}
    fn consume_statsd_metric_namespace(&mut self, _value: Option<String>) {}
    fn consume_statsd_metric_namespace_blacklist(&mut self, _value: Vec<String>) {}

    // ----- OTLP -----
    fn consume_otlp_config_metrics_enabled(&mut self, value: bool) {
        self.otlp.metrics_enabled = value;
    }
    fn consume_otlp_config_logs_enabled(&mut self, value: bool) {
        self.otlp.logs_enabled = value;
    }
    fn consume_otlp_config_traces_enabled(&mut self, value: bool) {
        self.otlp.traces_enabled = value;
    }
    fn consume_otlp_config_receiver_protocols_grpc_endpoint(&mut self, value: Option<String>) {
        self.otlp.grpc_endpoint = value.filter(|s| !s.is_empty()).map(MetaString::from);
    }
    fn consume_otlp_config_receiver_protocols_http_endpoint(&mut self, value: Option<String>) {
        self.otlp.http_endpoint = value.filter(|s| !s.is_empty()).map(MetaString::from);
    }
    fn consume_otlp_config_traces_internal_port(&mut self, value: i64) {
        self.otlp_traces_internal_port = value.clamp(0, u16::MAX as i64) as u16;
    }
    fn consume_data_plane_otlp_proxy_metrics_enabled(&mut self, value: bool) {
        self.otlp_proxy_metrics = value;
    }
    fn consume_data_plane_otlp_proxy_logs_enabled(&mut self, value: bool) {
        self.otlp_proxy_logs = value;
    }
    fn consume_data_plane_otlp_proxy_traces_enabled(&mut self, value: bool) {
        self.otlp_proxy_traces = value;
    }
    fn consume_otlp_config_receiver_protocols_grpc_transport(&mut self, _value: Option<String>) {}
    fn consume_otlp_config_receiver_protocols_grpc_max_recv_msg_size_mib(&mut self, _value: i64) {}

    // ----- Control-surface listen addresses -----
    fn consume_data_plane_api_listen_address(&mut self, value: Option<String>) {
        self.api_listen_address = value;
    }
    fn consume_data_plane_secure_api_listen_address(&mut self, value: Option<String>) {
        self.secure_api_listen_address = value;
    }

    // ----- Keys consumed at bootstrap, not part of the runtime native model -----
    fn consume_agent_ipc_grpc_max_message_size(&mut self, _value: i64) {}
    fn consume_cmd_port(&mut self, _value: i64) {}
    fn consume_vsock_addr(&mut self, _value: Option<String>) {}
    fn consume_bind_host(&mut self, _value: Option<String>) {}
    fn consume_data_plane_remote_agent_enabled(&mut self, _value: bool) {}
    fn consume_data_plane_use_new_config_stream_endpoint(&mut self, _value: bool) {}
    fn consume_data_plane_dogstatsd_aggregator_tag_filter_cache_capacity(&mut self, _value: i64) {}

    // ----- Keys ADP does not model (workload/proxy/provider metadata) -----
    fn consume_cri_connection_timeout(&mut self, _value: i64) {}
    fn consume_cri_query_timeout(&mut self, _value: i64) {}
    fn consume_origin_detection_unified(&mut self, _value: bool) {}
    fn consume_provider_kind(&mut self, _value: Option<String>) {}
    fn consume_observability_pipelines_worker_metrics_enabled(&mut self, _value: bool) {}
    fn consume_observability_pipelines_worker_metrics_url(&mut self, _value: Option<String>) {}
    fn consume_vector_metrics_enabled(&mut self, _value: bool) {}
    fn consume_vector_metrics_url(&mut self, _value: Option<String>) {}
}

#[cfg(test)]
mod tests {
    use agent_data_plane_config::RuntimeConfigLanguage;
    use bytesize::ByteSize;
    use saluki_component_config::common::CompressionKind;

    use super::*;

    #[test]
    fn translates_supported_keys_into_native_config() {
        // A Datadog source configuration with a representative set of supported keys set.
        let dd = DatadogConfiguration {
            dogstatsd_port: 8200,
            api_key: "test-api-key".to_string(),
            forwarder_timeout: 30,
            serializer_compressor_kind: "zstd".to_string(),
            skip_ssl_validation: true,
            ..Default::default()
        };

        let private = SalukiPrivateConfiguration::for_language(RuntimeConfigLanguage::DatadogAgent);
        let gates = PipelineGates {
            enabled: true,
            dogstatsd_enabled: true,
            checks_enabled: false,
            otlp_enabled: false,
        };

        let saluki = translate_datadog(&dd, &private, gates);

        // Witnessed Datadog keys land in their native destinations.
        assert_eq!(saluki.dogstatsd.source.port, 8200);
        assert_eq!(saluki.forwarder.datadog.request_timeout, Duration::from_secs(30));
        assert!(saluki.forwarder.datadog.tls.skip_ssl_validation);
        assert_eq!(saluki.metrics.datadog_encoder.compression.kind, CompressionKind::Zstd);

        // The primary endpoint is assembled from the API key plus the default intake URL.
        assert_eq!(saluki.forwarder.datadog.endpoints.len(), 1);
        let endpoint = &saluki.forwarder.datadog.endpoints[0];
        assert_eq!(endpoint.api_keys.len(), 1);
        assert_eq!(endpoint.api_keys[0].as_ref(), "test-api-key");
        assert_eq!(endpoint.dd_url.as_ref(), "https://app.datadoghq.com");

        // Pipeline gates flow through to the data-plane config.
        assert!(saluki.data_plane.enabled());
        assert!(saluki.data_plane.dogstatsd.enabled());
        assert!(!saluki.data_plane.otlp.enabled());

        // Saluki-private supplemental config is folded into the native model.
        assert_eq!(saluki.otlp.config.context_string_interner_bytes, ByteSize::mib(2));
    }

    #[test]
    fn no_api_key_yields_no_endpoints() {
        let dd = DatadogConfiguration::default();
        let private = SalukiPrivateConfiguration::for_language(RuntimeConfigLanguage::DatadogAgent);
        let saluki = translate_datadog(&dd, &private, PipelineGates::default());
        assert!(saluki.forwarder.datadog.endpoints.is_empty());
    }
}
