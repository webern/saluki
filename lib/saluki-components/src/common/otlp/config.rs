//! Shared OTLP receiver configuration.

use bytesize::ByteSize;
use datadog_agent_config::OtlpConfig as NativeOtlpConfig;
use facet::Facet;
use saluki_config::GenericConfiguration;
use saluki_error::GenericError;
use serde::Deserialize;

/// Saluki-private OTLP traces knobs that are not part of the Datadog Agent schema.
///
/// These keys (`otlp_config.traces.string_interner_size`,
/// `otlp_config.traces.ignore_missing_datadog_fields`,
/// `otlp_config.traces.enable_otlp_compute_top_level_by_span_kind`) are consumed by the OTLP traces
/// translator but are not in the vendored Datadog schema, so they have no destination in
/// `datadog_agent_config::OtlpConfig`. Until they move to `SalukiPrivateConfiguration` (a later
/// migration PR), the run.rs caller reads them from `GenericConfiguration` and passes them into the
/// native OTLP constructors via this struct. This keeps the migrated constructors free of any
/// `GenericConfiguration` dependency while preserving the existing behavior of these knobs.
#[derive(Clone, Copy, Debug)]
pub struct NativeTracesPrivateConfig {
    /// `otlp_config.traces.string_interner_size`: interner capacity for the traces translator.
    pub string_interner_bytes: ByteSize,
    /// `otlp_config.traces.ignore_missing_datadog_fields`: skip OTLP-semantic-convention fallback.
    pub ignore_missing_datadog_fields: bool,
    /// `otlp_config.traces.enable_otlp_compute_top_level_by_span_kind`: derive top-level by span kind.
    pub enable_otlp_compute_top_level_by_span_kind: bool,
}

impl Default for NativeTracesPrivateConfig {
    fn default() -> Self {
        Self {
            string_interner_bytes: default_traces_string_interner_size(),
            ignore_missing_datadog_fields: false,
            enable_otlp_compute_top_level_by_span_kind: default_enable_otlp_compute_top_level_by_span_kind(),
        }
    }
}

fn default_grpc_endpoint() -> String {
    "0.0.0.0:4317".to_string()
}

fn default_http_endpoint() -> String {
    "0.0.0.0:4318".to_string()
}

fn default_transport() -> String {
    "tcp".to_string()
}

fn default_max_recv_msg_size_mib() -> u64 {
    4
}

pub(crate) const fn default_traces_string_interner_size() -> ByteSize {
    ByteSize::kib(512)
}

/// Receiver configuration for OTLP endpoints.
///
/// This follows the Datadog Agent `otlp_config.receiver` structure.
#[derive(Deserialize, Debug, Default, Facet)]
#[cfg_attr(test, derive(PartialEq, serde::Serialize))]
pub struct Receiver {
    /// Protocol-specific receiver configuration.
    #[serde(default)]
    pub protocols: Protocols,
}

/// Protocol configuration for OTLP receiver.
#[derive(Deserialize, Debug, Default, Facet)]
#[cfg_attr(test, derive(PartialEq, serde::Serialize))]
pub struct Protocols {
    /// gRPC protocol configuration.
    #[serde(default)]
    pub grpc: GrpcConfig,

    /// HTTP protocol configuration.
    #[serde(default)]
    pub http: HttpConfig,
}

/// gRPC receiver configuration.
#[derive(Deserialize, Debug, Facet)]
#[cfg_attr(test, derive(PartialEq, serde::Serialize))]
pub struct GrpcConfig {
    /// The gRPC endpoint to listen on for OTLP requests.
    ///
    /// Defaults to `0.0.0.0:4317`.
    #[serde(default = "default_grpc_endpoint")]
    pub endpoint: String,

    /// The transport protocol to use for the gRPC listener.
    ///
    /// Defaults to `tcp`.
    #[serde(default = "default_transport")]
    pub transport: String,

    /// Maximum size (in MiB) of a gRPC message that can be received.
    ///
    /// Defaults to 4 MiB.
    #[serde(default = "default_max_recv_msg_size_mib", rename = "max_recv_msg_size_mib")]
    pub max_recv_msg_size_mib: u64,
}

/// HTTP receiver configuration.
#[derive(Deserialize, Debug, Facet)]
#[cfg_attr(test, derive(PartialEq, serde::Serialize))]
pub struct HttpConfig {
    /// The HTTP endpoint to listen on for OTLP requests.
    ///
    /// Defaults to `0.0.0.0:4318`.
    #[serde(default = "default_http_endpoint")]
    pub endpoint: String,

    /// The transport protocol to use for the HTTP listener.
    ///
    /// Defaults to `tcp`.
    #[serde(default = "default_transport")]
    pub transport: String,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            endpoint: default_grpc_endpoint(),
            transport: default_transport(),
            max_recv_msg_size_mib: default_max_recv_msg_size_mib(),
        }
    }
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            endpoint: default_http_endpoint(),
            transport: default_transport(),
        }
    }
}

/// OTLP configuration.
///
/// This mirrors the Agent's `otlp_config` and contains configuration for
/// the OTLP receiver as well as signal-specific settings (metrics, logs, traces).
#[derive(Deserialize, Debug, Default)]
#[cfg_attr(test, derive(PartialEq, serde::Serialize))]
pub struct OtlpConfig {
    /// OTLP receiver configuration.
    #[serde(default)]
    pub receiver: Receiver,

    /// Metrics-specific OTLP configuration.
    #[serde(default)]
    pub metrics: MetricsConfig,

    /// Logs-specific OTLP configuration.
    #[serde(default)]
    pub logs: LogsConfig,

    /// Traces-specific OTLP configuration.
    #[serde(default)]
    pub traces: TracesConfig,
}

impl OtlpConfig {
    /// Builds the component OTLP config from native translated config plus the Saluki-private traces
    /// knobs.
    ///
    /// `native` carries the Datadog-schema OTLP keys (receiver endpoints/transport/size, the
    /// metrics/logs/traces enable flags, the traces internal port, and the env-resolved probabilistic
    /// sampling percentage). `traces_private` carries the non-schema traces knobs the run.rs caller
    /// reads from `GenericConfiguration` (see [`NativeTracesPrivateConfig`]).
    ///
    /// The receiver HTTP transport is not a Datadog-schema key, so it is fixed to `tcp` here, matching
    /// the component default and the only transport the OTLP servers support.
    ///
    /// Behavior change on cutover: `metrics`/`logs`/`traces` `enabled` now follow the Datadog Agent
    /// schema defaults carried in `native` (metrics on, logs OFF, traces on when unset). The legacy
    /// `LogsConfig` serde default wrongly defaulted logs to on; sourcing from the authoritative schema
    /// disables OTLP logs by default when `otlp_config.logs.enabled` is unset, matching the Core Agent.
    pub fn from_native(native: &NativeOtlpConfig, traces_private: NativeTracesPrivateConfig) -> Self {
        Self {
            receiver: Receiver {
                protocols: Protocols {
                    grpc: GrpcConfig {
                        endpoint: native.grpc.endpoint.clone(),
                        transport: native.grpc.transport.clone(),
                        max_recv_msg_size_mib: native.grpc.max_recv_msg_size_mib,
                    },
                    http: HttpConfig {
                        endpoint: native.http.endpoint.clone(),
                        transport: default_transport(),
                    },
                },
            },
            metrics: MetricsConfig {
                enabled: native.metrics_enabled,
            },
            logs: LogsConfig {
                enabled: native.logs_enabled,
            },
            traces: TracesConfig::from_native(native, traces_private),
        }
    }
}

/// Configuration for OTLP logs processing.
#[derive(Deserialize, Debug)]
#[cfg_attr(test, derive(PartialEq, serde::Serialize))]
pub struct LogsConfig {
    /// Whether to enable OTLP logs support.
    ///
    /// Defaults to `true`.
    #[serde(default = "default_logs_enabled")]
    pub enabled: bool,
}

fn default_logs_enabled() -> bool {
    true
}

impl Default for LogsConfig {
    fn default() -> Self {
        Self {
            enabled: default_logs_enabled(),
        }
    }
}

/// Configuration for OTLP metrics processing.
#[derive(Deserialize, Debug)]
#[cfg_attr(test, derive(PartialEq, serde::Serialize))]
pub struct MetricsConfig {
    /// Whether to enable OTLP metrics support.
    ///
    /// Defaults to `true`.
    #[serde(default = "default_metrics_enabled")]
    pub enabled: bool,
}

fn default_metrics_enabled() -> bool {
    true
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: default_metrics_enabled(),
        }
    }
}

/// Configuration for OTLP traces processing.
///
/// Mirrors the Agent's `otlp_config.traces` configuration.
#[derive(Clone, Deserialize, Debug)]
#[cfg_attr(test, derive(PartialEq, serde::Serialize))]
pub struct TracesConfig {
    /// Whether to enable OTLP traces support.
    ///
    /// Defaults to `true`.
    #[serde(default = "default_traces_enabled")]
    pub enabled: bool,

    /// Whether to skip deriving Datadog fields from standard OTLP attributes.
    ///
    /// When true, only uses explicit `datadog.*` prefixed attributes and skips
    /// fallback resolution from OTLP semantic conventions.
    ///
    /// Corresponds to `otlp_config.traces.ignore_missing_datadog_fields` in the Agent.
    ///
    /// Defaults to `false`.
    #[serde(default)]
    pub ignore_missing_datadog_fields: bool,

    /// When true, `_top_level` and `_dd.measured` are derived using the OTLP span kind.
    ///
    /// Corresponds to the `enable_otlp_compute_top_level_by_span_kind` feature flag
    /// in the Agent's `apm_config.features`.
    ///
    /// Defaults to `true`.
    #[serde(default = "default_enable_otlp_compute_top_level_by_span_kind")]
    pub enable_otlp_compute_top_level_by_span_kind: bool,

    /// Probabilistic sampler configuration for OTLP traces.
    ///
    /// Corresponds to `otlp_config.traces.probabilistic_sampler` in the Agent.
    #[serde(default)]
    pub probabilistic_sampler: ProbabilisticSampler,

    /// Total size of the string interner used for OTLP traces.
    ///
    /// Defaults to 512 KiB.
    #[serde(rename = "string_interner_size", default = "default_traces_string_interner_size")]
    pub string_interner_bytes: ByteSize,

    /// The internal port on the Core Agent to forward traces to.
    ///
    /// Defaults to 5003.
    #[serde(default = "default_internal_port")]
    #[allow(unused)]
    pub internal_port: u16,
}

const fn default_internal_port() -> u16 {
    5003
}

/// Configuration for OTLP traces probabilistic sampling.
#[derive(Clone, Deserialize, Debug)]
#[cfg_attr(test, derive(PartialEq, serde::Serialize))]
pub struct ProbabilisticSampler {
    /// Percentage of traces to ingest (0, 100].
    ///
    /// Invalid values (<= 0 || > 100) are disregarded and the default is used.
    ///
    /// Corresponds to `otlp_config.traces.probabilistic_sampler.sampling_percentage` in the Agent.
    ///
    /// Defaults to 100.0 (100% sampling).
    #[serde(default = "default_sampling_percentage")]
    pub sampling_percentage: f64,
}

const fn default_sampling_percentage() -> f64 {
    100.0
}

impl Default for ProbabilisticSampler {
    fn default() -> Self {
        Self {
            sampling_percentage: default_sampling_percentage(),
        }
    }
}

const fn default_enable_otlp_compute_top_level_by_span_kind() -> bool {
    true
}

fn default_traces_enabled() -> bool {
    true
}

impl TracesConfig {
    /// Builds the component traces config from native translated config plus Saluki-private knobs.
    ///
    /// The schema keys (`enabled`, `internal_port`, and the probabilistic sampler percentage) come
    /// from `native`; the sampling percentage there already reflects any
    /// `DD_OTLP_CONFIG_TRACES_PROBABILISTIC_SAMPLER_SAMPLING_PERCENTAGE` env override applied at the
    /// translation boundary (see `bin/agent-data-plane` run path), so no separate
    /// [`apply_env_overrides`][Self::apply_env_overrides] step is needed on the migrated path. The
    /// non-schema knobs come from `private`.
    pub fn from_native(native: &NativeOtlpConfig, private: NativeTracesPrivateConfig) -> Self {
        Self {
            enabled: native.traces_enabled,
            ignore_missing_datadog_fields: private.ignore_missing_datadog_fields,
            enable_otlp_compute_top_level_by_span_kind: private.enable_otlp_compute_top_level_by_span_kind,
            probabilistic_sampler: ProbabilisticSampler {
                sampling_percentage: native.traces_sampling_percentage,
            },
            string_interner_bytes: private.string_interner_bytes,
            internal_port: native.traces_internal_port,
        }
    }

    /// Applies env var overrides for keys whose `DD_`-stripped flat form can't reach the nested
    /// struct through normal serde deserialization.
    ///
    /// `DD_OTLP_CONFIG_TRACES_PROBABILISTIC_SAMPLER_SAMPLING_PERCENTAGE` strips to flat Figment key
    /// `otlp_config_traces_probabilistic_sampler_sampling_percentage`. KEY_ALIASES ensures YAML and
    /// env var land on the same key, but a nested struct can't see a flat key—so we read it
    /// explicitly and override.
    pub(crate) fn apply_env_overrides(&mut self, config: &GenericConfiguration) -> Result<(), GenericError> {
        if let Some(pct) =
            config.try_get_typed::<f64>("otlp_config_traces_probabilistic_sampler_sampling_percentage")?
        {
            self.probabilistic_sampler.sampling_percentage = pct;
        }
        Ok(())
    }
}

impl Default for TracesConfig {
    fn default() -> Self {
        Self {
            enabled: default_traces_enabled(),
            ignore_missing_datadog_fields: false,
            enable_otlp_compute_top_level_by_span_kind: default_enable_otlp_compute_top_level_by_span_kind(),
            probabilistic_sampler: ProbabilisticSampler::default(),
            string_interner_bytes: default_traces_string_interner_size(),
            internal_port: default_internal_port(),
        }
    }
}
