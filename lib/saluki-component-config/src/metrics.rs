//! Native configuration for the metrics pipeline (enrichment + Datadog encoder).

use stringtheory::MetaString;

use crate::common::CompressionConfig;

/// Native configuration for the metrics enrichment transform.
///
/// Host tags and origin enrichment are supplied at runtime through an environment provider /
/// Datadog Agent connection rather than read from a configuration map, so this struct only carries
/// the static enrichment knobs.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MetricsEnrichmentConfig {
    /// Extra host tags applied to every metric.
    pub host_tags: Vec<MetaString>,

    /// Whether origin-detection enrichment is enabled.
    pub origin_detection_enabled: bool,
}

/// Native configuration for the Datadog metrics encoder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatadogMetricsEncoderConfig {
    /// Maximum number of metrics per outbound payload.
    pub max_metrics_per_payload: usize,

    /// Maximum compressed payload size in bytes.
    pub max_payload_size: usize,

    /// Maximum uncompressed payload size in bytes.
    pub max_uncompressed_payload_size: usize,

    /// Maximum compressed series payload size in bytes.
    pub max_series_payload_size: usize,

    /// Maximum uncompressed series payload size in bytes.
    pub max_series_uncompressed_payload_size: usize,

    /// Maximum number of series points per payload.
    pub max_series_points_per_payload: usize,

    /// Flush timeout in seconds.
    pub flush_timeout_secs: u64,

    /// Payload compression settings.
    pub compression: CompressionConfig,

    /// Whether to use the v2 series API.
    pub use_v2_api_series: bool,

    /// Whether outbound payloads are logged (debugging).
    pub log_payloads: bool,
}

impl Default for DatadogMetricsEncoderConfig {
    fn default() -> Self {
        Self {
            max_metrics_per_payload: 0,
            max_payload_size: 3_200_000,
            max_uncompressed_payload_size: 62_914_560,
            max_series_payload_size: 512_000,
            max_series_uncompressed_payload_size: 5_242_880,
            max_series_points_per_payload: 10_000,
            flush_timeout_secs: 2,
            compression: CompressionConfig::default(),
            use_v2_api_series: true,
            log_payloads: false,
        }
    }
}
