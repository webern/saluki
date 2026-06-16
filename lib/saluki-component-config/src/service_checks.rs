//! Native configuration for the Datadog service checks encoder.

use crate::common::CompressionConfig;

/// Native configuration for the Datadog service checks encoder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatadogServiceChecksEncoderConfig {
    /// Maximum compressed payload size in bytes.
    pub max_payload_size: usize,

    /// Maximum uncompressed payload size in bytes.
    pub max_uncompressed_payload_size: usize,

    /// Payload compression settings.
    pub compression: CompressionConfig,

    /// Whether outbound payloads are logged (debugging).
    pub log_payloads: bool,
}

impl Default for DatadogServiceChecksEncoderConfig {
    fn default() -> Self {
        Self {
            max_payload_size: 3_200_000,
            max_uncompressed_payload_size: 62_914_560,
            compression: CompressionConfig::default(),
            log_payloads: false,
        }
    }
}
