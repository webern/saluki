//! Cross-cutting native configuration building blocks shared by several components.

use std::time::Duration;

use stringtheory::MetaString;

/// Outbound TLS settings for HTTP clients.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TlsClientConfig {
    /// Whether server certificate validation is skipped.
    pub skip_ssl_validation: bool,

    /// Minimum negotiated TLS version (already clamped to a version the runtime supports).
    pub min_tls_version: TlsMinimumVersion,

    /// Optional path for an SSL key log file used for debugging.
    pub ssl_key_log_file: Option<MetaString>,
}

impl Default for TlsClientConfig {
    fn default() -> Self {
        Self {
            skip_ssl_validation: false,
            min_tls_version: TlsMinimumVersion::Tls1_2,
            ssl_key_log_file: None,
        }
    }
}

/// Minimum negotiated outbound TLS version.
///
/// The translator is responsible for clamping unsupported source-language values (for example the
/// Datadog `min_tls_version` values `tlsv1.0`/`tlsv1.1`) to a version the runtime can actually
/// negotiate, so this native type only models the versions ADP supports.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TlsMinimumVersion {
    /// TLS 1.2.
    Tls1_2,
    /// TLS 1.3.
    Tls1_3,
}

/// Payload compression settings for outbound encoders.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompressionConfig {
    /// Compression algorithm.
    pub kind: CompressionKind,

    /// Zstd compression level (only meaningful when `kind` is `Zstd`).
    pub zstd_level: i32,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            kind: CompressionKind::Zstd,
            zstd_level: 3,
        }
    }
}

/// Supported payload compression algorithms.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompressionKind {
    /// Zstandard.
    Zstd,
    /// Zlib/deflate.
    Zlib,
}

/// Retry/backoff settings for outbound transports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetryConfig {
    /// Maximum backoff between retries.
    pub max_backoff: Duration,

    /// Base interval used to compute exponential backoff.
    pub base_backoff: Duration,

    /// Whether failed payloads are persisted to disk for later retry.
    pub disk_persistence_enabled: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_backoff: Duration::from_secs(30),
            base_backoff: Duration::from_secs(2),
            disk_persistence_enabled: false,
        }
    }
}

/// A single resolved intake endpoint plus the API keys to send to it.
///
/// The API key is a resolved snapshot. Runtime refresh of the API key is a separate typed
/// capability handled by the configuration system, not a value re-read from a configuration map by
/// the component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointConfig {
    /// Intake base URL (for example `https://app.datadoghq.com`).
    pub dd_url: MetaString,

    /// API keys authorized for this endpoint.
    pub api_keys: Vec<MetaString>,
}

impl EndpointConfig {
    /// Creates an endpoint config with a single API key.
    pub fn new(dd_url: impl Into<MetaString>, api_key: impl Into<MetaString>) -> Self {
        Self {
            dd_url: dd_url.into(),
            api_keys: vec![api_key.into()],
        }
    }
}
