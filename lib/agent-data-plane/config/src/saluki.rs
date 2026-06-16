//! ADP-native runtime configuration.
//!
//! [`SalukiConfiguration`] is the complete, typed, ADP-native runtime configuration after
//! translation. Components consume it — never source maps, never source-language schema types.
//! Because it embeds `saluki-component-config` structs, the translator's output *is* the
//! component's input; topology assembly hands each component its slice directly.

use saluki_component_config::{
    AggregateConfig, ApmStatsEncoderConfig, ChecksConfig, DatadogEventsEncoderConfig, DatadogForwarderConfig,
    DatadogLogsEncoderConfig, DatadogMetricsEncoderConfig, DatadogServiceChecksEncoderConfig,
    DatadogTracesEncoderConfig, DogStatsDConfig, DogStatsDDebugLogConfig, DogStatsDMapperConfig,
    MetricsEnrichmentConfig, MultiRegionFailoverConfig, OtlpConfig, PrefixFilterConfig, TagFilterlistConfig,
    TraceObfuscationConfig, TraceSamplerConfig, TracesEnrichmentConfig,
};
use saluki_io::net::ListenAddress;

use crate::logging::RuntimeLoggingConfig;
use crate::private::WorkloadPrivateConfig;

/// Complete ADP-native runtime configuration produced by a source-language translator.
#[derive(Clone, Debug)]
pub struct SalukiConfiguration {
    /// Runtime logging configuration (reloaded once the authoritative config arrives).
    pub logging: RuntimeLoggingConfig,

    /// Top-level data-plane gating and control-surface addresses.
    pub data_plane: DataPlaneConfig,

    /// Process memory-bounds configuration.
    pub memory: MemoryConfig,

    /// Outbound forwarder configuration.
    pub forwarder: ForwarderConfigs,

    /// Metrics pipeline configuration.
    pub metrics: MetricsConfigs,

    /// Logs pipeline configuration.
    pub logs: LogsConfigs,

    /// Events pipeline configuration.
    pub events: EventsConfigs,

    /// Service checks pipeline configuration.
    pub service_checks: ServiceChecksConfigs,

    /// Traces / APM pipeline configuration.
    pub traces: TracesConfigs,

    /// Checks IPC source configuration.
    pub checks: ChecksConfigs,

    /// DogStatsD pipeline configuration.
    pub dogstatsd: DogStatsDConfigs,

    /// OTLP ingest configuration.
    pub otlp: OtlpConfigs,

    /// Workload-metadata (environment provider) tuning knobs.
    pub workload: WorkloadPrivateConfig,
}

/// Process memory-bounds configuration.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MemoryConfig {
    /// Overall process memory limit in bytes, if configured.
    pub memory_limit_bytes: Option<u64>,

    /// Slop factor (0.0..=1.0) withheld from the limit, or `None` to use the application default.
    pub slop_factor: Option<f64>,

    /// Whether the global memory limiter is enabled, or `None` to use the application default.
    pub enable_global_limiter: Option<bool>,
}

/// A simple enable/disable gate for a pipeline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PipelineGate {
    enabled: bool,
}

impl PipelineGate {
    /// Creates a gate.
    pub const fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Returns whether the pipeline is enabled.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
}

/// Top-level data-plane gating and control-surface addresses.
#[derive(Clone, Debug)]
pub struct DataPlaneConfig {
    /// Whether the data plane is enabled at all.
    pub enabled: bool,

    /// Unprivileged control API listen address.
    pub api_listen_address: ListenAddress,

    /// Privileged (secure) control API listen address.
    pub secure_api_listen_address: ListenAddress,

    /// DogStatsD pipeline gate.
    pub dogstatsd: PipelineGate,

    /// Checks pipeline gate.
    pub checks: PipelineGate,

    /// OTLP pipeline gate.
    pub otlp: PipelineGate,

    /// Whether OTLP proxy mode is active (vs native OTLP handling).
    pub otlp_proxy_enabled: bool,

    /// Whether OTLP proxy mode proxies traces (vs decoding them natively).
    pub otlp_proxy_traces: bool,
}

impl DataPlaneConfig {
    /// Whether the data plane is enabled.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Whether any data pipeline is enabled.
    pub const fn data_pipelines_enabled(&self) -> bool {
        self.checks.enabled() || self.dogstatsd.enabled() || self.otlp.enabled()
    }

    /// Whether the outbound Datadog forwarder is required.
    pub const fn requires_datadog_forwarder(&self) -> bool {
        self.data_pipelines_enabled()
    }

    /// Whether the baseline metrics pipeline is required.
    pub const fn metrics_pipeline_required(&self) -> bool {
        self.checks.enabled() || self.dogstatsd.enabled() || (self.otlp.enabled() && !self.otlp_proxy_enabled)
    }

    /// Whether the baseline logs pipeline is required.
    pub const fn logs_pipeline_required(&self) -> bool {
        self.checks.enabled() || (self.otlp.enabled() && !self.otlp_proxy_enabled)
    }

    /// Whether the baseline events pipeline is required.
    pub const fn events_pipeline_required(&self) -> bool {
        self.checks.enabled() || self.dogstatsd.enabled()
    }

    /// Whether the baseline service checks pipeline is required.
    pub const fn service_checks_pipeline_required(&self) -> bool {
        self.checks.enabled() || self.dogstatsd.enabled()
    }

    /// Whether the traces pipeline is required.
    pub const fn traces_pipeline_required(&self) -> bool {
        self.otlp.enabled() && (!self.otlp_proxy_enabled || !self.otlp_proxy_traces)
    }
}

/// Outbound forwarder configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForwarderConfigs {
    /// The primary Datadog forwarder.
    pub datadog: DatadogForwarderConfig,
}

/// Metrics pipeline configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct MetricsConfigs {
    /// Metrics enrichment transform configuration.
    pub enrichment: MetricsEnrichmentConfig,

    /// Datadog metrics encoder configuration.
    pub datadog_encoder: DatadogMetricsEncoderConfig,

    /// Multi-region failover configuration, when enabled.
    pub multi_region_failover: Option<MultiRegionFailoverConfig>,
}

/// Logs pipeline configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogsConfigs {
    /// Datadog logs encoder configuration.
    pub datadog_encoder: DatadogLogsEncoderConfig,
}

/// Events pipeline configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventsConfigs {
    /// Datadog events encoder configuration.
    pub datadog_encoder: DatadogEventsEncoderConfig,
}

/// Service checks pipeline configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceChecksConfigs {
    /// Datadog service checks encoder configuration.
    pub datadog_encoder: DatadogServiceChecksEncoderConfig,
}

/// Traces / APM pipeline configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct TracesConfigs {
    /// Traces enrichment transform configuration.
    pub enrichment: TracesEnrichmentConfig,

    /// Trace sampler configuration.
    pub sampler: TraceSamplerConfig,

    /// Trace obfuscation configuration.
    pub obfuscation: TraceObfuscationConfig,

    /// APM stats encoder configuration.
    pub apm_stats_encoder: ApmStatsEncoderConfig,

    /// Datadog traces encoder configuration.
    pub datadog_encoder: DatadogTracesEncoderConfig,
}

/// Checks IPC source configuration.
#[derive(Clone, Debug)]
pub struct ChecksConfigs {
    /// Checks IPC source configuration.
    pub ipc: ChecksConfig,
}

/// DogStatsD pipeline configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct DogStatsDConfigs {
    /// DogStatsD source configuration.
    pub source: DogStatsDConfig,

    /// DogStatsD mapper transform configuration.
    pub mapper: DogStatsDMapperConfig,

    /// Prefix/blocklist filter configuration.
    pub prefix_filter: PrefixFilterConfig,

    /// Tag filterlist configuration.
    pub tag_filterlist: TagFilterlistConfig,

    /// Aggregation transform configuration.
    pub aggregate: AggregateConfig,

    /// Debug-log destination configuration, when enabled.
    pub debug_log: Option<DogStatsDDebugLogConfig>,
}

/// OTLP ingest configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct OtlpConfigs {
    /// OTLP source/proxy configuration.
    pub config: OtlpConfig,
}
