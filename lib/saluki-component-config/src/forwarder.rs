//! Native configuration for outbound Datadog forwarders.

use std::time::Duration;

use stringtheory::MetaString;

use crate::common::{EndpointConfig, RetryConfig, TlsClientConfig};

/// Native configuration for the Datadog forwarder.
///
/// This is the resolved, typed shape the forwarder component is built from. It carries no Datadog
/// key names and no live configuration map; runtime API-key refresh is supplied by a separate typed
/// capability handle owned by the configuration system.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatadogForwarderConfig {
    /// Primary intake endpoint plus any additional dual-ship endpoints.
    pub endpoints: Vec<EndpointConfig>,

    /// Maximum number of concurrent in-flight requests per endpoint.
    pub endpoint_concurrency: usize,

    /// Multiplier applied to per-endpoint worker counts.
    pub endpoint_concurrency_multiplier: usize,

    /// Per-request timeout.
    pub request_timeout: Duration,

    /// High-priority queue buffer size.
    pub endpoint_buffer_size: usize,

    /// Interval at which idle connections are recycled.
    pub connection_reset_interval: Option<Duration>,

    /// Whether arbitrary tags are permitted on outbound payloads.
    pub allow_arbitrary_tags: bool,

    /// Outbound TLS settings.
    pub tls: TlsClientConfig,

    /// Retry/backoff settings.
    pub retry: RetryConfig,
}

impl Default for DatadogForwarderConfig {
    fn default() -> Self {
        Self {
            endpoints: Vec::new(),
            endpoint_concurrency: 1,
            endpoint_concurrency_multiplier: 1,
            request_timeout: Duration::from_secs(20),
            endpoint_buffer_size: 100,
            connection_reset_interval: None,
            allow_arbitrary_tags: false,
            tls: TlsClientConfig::default(),
            retry: RetryConfig::default(),
        }
    }
}

/// Native configuration for the Multi-Region Failover (MRF) metrics path.
///
/// When present, ADP stands up a parallel metrics gateway, encoder, and forwarder that dual-ship a
/// filtered metric set to a failover region.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiRegionFailoverConfig {
    /// Whether failover metrics shipping is currently active.
    pub failover_metrics: bool,

    /// Metric-name allowlist applied at the gateway.
    pub metric_allowlist: Vec<MetaString>,

    /// The failover forwarder configuration (its own endpoint and API key).
    pub forwarder: DatadogForwarderConfig,
}
