//! Native configuration for the Datadog logs encoder.

use crate::common::CompressionConfig;

/// Native configuration for the Datadog logs encoder.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DatadogLogsEncoderConfig {
    /// Payload compression settings.
    pub compression: CompressionConfig,
}
