//! Native configuration for the traces / APM pipeline.

use stringtheory::MetaString;

use crate::common::CompressionConfig;

/// Native configuration for the traces enrichment transform.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TracesEnrichmentConfig {
    /// Default environment applied to spans missing an `env`.
    pub default_env: MetaString,
}

/// Native configuration for the trace sampler transform.
#[derive(Clone, Debug, PartialEq)]
pub struct TraceSamplerConfig {
    /// Target sampled traces per second.
    pub target_traces_per_second: f64,

    /// Error-sampled traces per second.
    pub errors_per_second: f64,

    /// Whether error sampling is enabled.
    pub error_sampling_enabled: bool,

    /// Whether the rare sampler is enabled.
    pub rare_sampler_enabled: bool,

    /// OTLP ingest sampling rate (0.0..=1.0).
    pub otlp_sampling_rate: f64,
}

impl Default for TraceSamplerConfig {
    fn default() -> Self {
        Self {
            target_traces_per_second: 10.0,
            errors_per_second: 10.0,
            error_sampling_enabled: true,
            rare_sampler_enabled: true,
            otlp_sampling_rate: 1.0,
        }
    }
}

/// Native configuration for the trace obfuscation transform.
///
/// The detailed obfuscation rule set is intentionally summarized here; the spike models the
/// load-bearing toggles a translator must populate, leaving the exhaustive rule structure to a
/// later fidelity pass.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TraceObfuscationConfig {
    /// Whether SQL query obfuscation is enabled.
    pub obfuscate_sql: bool,

    /// Whether credit-card scrubbing is enabled.
    pub scrub_credit_cards: bool,

    /// Tag names whose values must be removed entirely.
    pub removed_tags: Vec<MetaString>,
}

/// Native configuration for the APM stats encoder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApmStatsEncoderConfig {
    /// Flush timeout in seconds.
    pub flush_timeout_secs: u64,

    /// Default environment reported in stats payloads.
    pub default_env: MetaString,
}

impl Default for ApmStatsEncoderConfig {
    fn default() -> Self {
        Self {
            flush_timeout_secs: 2,
            default_env: MetaString::empty(),
        }
    }
}

/// Native configuration for the Datadog traces encoder.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DatadogTracesEncoderConfig {
    /// Payload compression settings.
    pub compression: CompressionConfig,

    /// Default environment reported in trace payloads.
    pub default_env: MetaString,
}
