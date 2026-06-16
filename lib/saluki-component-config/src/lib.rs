//! Component-native configuration structs shared by ADP translators and components.

use saluki_io::net::ListenAddress;

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
