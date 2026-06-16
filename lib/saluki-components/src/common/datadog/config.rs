use std::time::Duration;

use facet::Facet;
use saluki_config::GenericConfiguration;
use saluki_error::GenericError;
use saluki_io::net::client::http::{HttpProtocol, TlsMinimumVersion};
use serde::Deserialize;
use tracing::warn;

use super::{
    endpoints::{EndpointConfiguration, EndpointRoute, RoutableEndpoint},
    proxy::ProxyConfiguration,
    retry::RetryConfiguration,
};

const fn default_endpoint_concurrency() -> usize {
    10
}

const fn default_endpoint_concurrency_multiplier() -> usize {
    1
}

const fn default_request_timeout_secs() -> u64 {
    20
}

const fn default_endpoint_buffer_size() -> usize {
    100
}

const fn default_forwarder_connection_reset_interval() -> u64 {
    0
}

const fn default_api_key_validation_interval_mins() -> u64 {
    60
}

const fn default_api_key_validation_interval_config_mins() -> i64 {
    default_api_key_validation_interval_mins() as i64
}

const MIN_TLS_VERSION_TLS12: &str = "tlsv1.2";
const MIN_TLS_VERSION_TLS13: &str = "tlsv1.3";

fn default_min_tls_version() -> String {
    MIN_TLS_VERSION_TLS12.to_string()
}

/// HTTP protocol selection for the Datadog forwarder.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Facet)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(test, derive(serde::Serialize))]
pub enum ForwarderHttpProtocol {
    /// Automatically negotiate HTTP/2 with HTTP/1.1 fallback.
    #[default]
    Auto,

    /// Use HTTP/1.1 only.
    Http1,
}

impl From<ForwarderHttpProtocol> for HttpProtocol {
    fn from(protocol: ForwarderHttpProtocol) -> Self {
        match protocol {
            ForwarderHttpProtocol::Auto => Self::Auto,
            ForwarderHttpProtocol::Http1 => Self::Http1,
        }
    }
}

/// OPW metrics endpoint configuration.
#[derive(Clone, Default, Deserialize, Facet)]
#[cfg_attr(test, derive(Debug, PartialEq, serde::Serialize))]
pub(crate) struct OpwMetricsConfiguration {
    /// Enables routing all metrics to Observability Pipelines Worker.
    ///
    /// Defaults to `false`.
    #[serde(default, rename = "observability_pipelines_worker_metrics_enabled")]
    observability_pipelines_worker_enabled: bool,

    /// Endpoint of the Observability Pipelines Worker instance to route metrics to.
    ///
    /// Defaults to unset.
    #[serde(default, rename = "observability_pipelines_worker_metrics_url")]
    observability_pipelines_worker_url: String,

    /// Enables routing all metrics to Vector.
    ///
    /// Deprecated in favor of `observability_pipelines_worker.metrics.enabled`.
    ///
    /// Defaults to `false`.
    #[serde(default, rename = "vector_metrics_enabled")]
    vector_enabled: bool,

    /// Endpoint of the Vector instance to route metrics to.
    ///
    /// Deprecated in favor of `observability_pipelines_worker.metrics.url`.
    ///
    /// Defaults to unset.
    #[serde(default, rename = "vector_metrics_url")]
    vector_url: String,
}

struct SelectedOpwMetricsEndpoint<'a> {
    enabled_key: &'static str,
    url_key: &'static str,
    url: &'a str,
}

impl OpwMetricsConfiguration {
    fn selected_endpoint(&self) -> Option<SelectedOpwMetricsEndpoint<'_>> {
        if self.observability_pipelines_worker_enabled {
            return Some(SelectedOpwMetricsEndpoint {
                enabled_key: "observability_pipelines_worker.metrics.enabled",
                url_key: "observability_pipelines_worker.metrics.url",
                url: &self.observability_pipelines_worker_url,
            });
        }

        if self.vector_enabled {
            return Some(SelectedOpwMetricsEndpoint {
                enabled_key: "vector.metrics.enabled",
                url_key: "vector.metrics.url",
                url: &self.vector_url,
            });
        }

        None
    }
}

/// Forwarder configuration based on the Datadog Agent's forwarder configuration.
///
/// This adapter provides a simple way to utilize the existing configuration values that are passed to the Datadog
/// Agent, which are used to control the behavior of its forwarder, such as retries and concurrency, in conjunction with
/// with existing primitives, as such retry policies in [`saluki_io::util::retry`].
#[derive(Clone, Deserialize, Facet)]
#[cfg_attr(test, derive(Debug, PartialEq, serde::Serialize))]
pub struct ForwarderConfiguration {
    /// Maximum number of concurrent requests for an individual endpoint.
    ///
    /// Defaults to 10. If set to 0, request concurrency is clamped to 1.
    #[serde(
        default = "default_endpoint_concurrency",
        rename = "forwarder_max_concurrent_requests"
    )]
    endpoint_concurrency: usize,

    /// Multiplier for endpoint request concurrency.
    ///
    /// Defaults to 1. This value also sizes the HTTP idle connection pool. If set to 0, idle connection retention is
    /// disabled and the concurrency multiplier is treated as 1. This setting does not create worker tasks.
    #[serde(
        default = "default_endpoint_concurrency_multiplier",
        rename = "forwarder_num_workers"
    )]
    endpoint_concurrency_multiplier: usize,

    /// Request timeout, in seconds.
    ///
    /// Defaults to 20 seconds.
    #[serde(default = "default_request_timeout_secs", rename = "forwarder_timeout")]
    request_timeout_secs: u64,

    /// Maximum number of pending requests for an individual endpoint.
    ///
    /// Defaults to 100.
    #[serde(default = "default_endpoint_buffer_size", rename = "forwarder_high_prio_buffer_size")]
    endpoint_buffer_size: usize,

    /// Endpoint configuration.
    #[serde(flatten)]
    pub(crate) endpoint: EndpointConfiguration,

    /// Retry configuration.
    #[serde(flatten)]
    retry: RetryConfiguration,

    /// Proxy configuration.
    #[serde(flatten)]
    proxy: Option<ProxyConfiguration>,

    /// OPW metrics routing configuration.
    #[serde(flatten)]
    opw_metrics: OpwMetricsConfiguration,

    /// HTTP protocol selection for outgoing forwarder requests.
    ///
    /// Defaults to `auto`, which negotiates HTTP/2 with HTTP/1.1 fallback. Set to `http1` to force HTTP/1.1 only.
    #[serde(default, rename = "forwarder_http_protocol")]
    http_protocol: ForwarderHttpProtocol,

    /// Connection reset interval, in seconds.
    ///
    /// Defaults to 0.
    #[serde(
        default = "default_forwarder_connection_reset_interval",
        rename = "forwarder_connection_reset_interval"
    )]
    connection_reset_interval_secs: u64,

    /// Whether to disable TLS certificate validation for Datadog intake forwarding.
    ///
    /// Defaults to `false`. If set to `true`, HTTPS clients built for the shared Datadog forwarder accept invalid
    /// server certificates. Only deployments that intentionally route Datadog intake traffic through endpoints with
    /// invalid or self-signed certificates should enable this.
    #[serde(default)]
    skip_ssl_validation: bool,

    /// File path to write TLS key material to for all HTTPS connections to the
    /// Datadog backend.
    ///
    /// When non-empty, enables the logging of TLS key material to the given file path,
    /// in the [NSS Key Log][nss_key_log] format, which can be used for debugging TLS
    /// issues, as well as decrypting captured TLS traffic in tools such as Wireshark.
    ///
    /// Defaults to empty.
    ///
    /// [nss_key_log]: https://nss-crypto.org/reference/security/nss/legacy/key_log_format/index.html
    #[serde(default)]
    sslkeylogfile: String,

    /// Minimum TLS protocol version for Datadog intake forwarding.
    ///
    /// Defaults to TLS 1.2. TLS 1.0 and TLS 1.1 are accepted for compatibility with core Agent configuration, but
    /// Saluki clamps them to TLS 1.2 because rustls does not support older protocol versions.
    #[serde(default = "default_min_tls_version")]
    min_tls_version: String,

    /// Parsed minimum TLS protocol version for Datadog intake forwarding.
    #[serde(skip)]
    #[facet(opaque)]
    parsed_min_tls_version: TlsMinimumVersion,

    /// Whether to signal that the backend should allow arbitrary tag values.
    ///
    /// Defaults to `false`. If set to `true`, the Datadog forwarder adds `Allow-Arbitrary-Tag-Value: true` to every
    /// outbound intake request. The data plane does not perform local tag validation based on this setting.
    #[serde(default)]
    allow_arbitrary_tags: bool,

    /// API key validation interval, in minutes.
    ///
    /// All values that are less than or equal to zero will be ignored, and the default
    /// value will be used.
    ///
    /// Defaults to 60 minutes.
    #[serde(
        default = "default_api_key_validation_interval_config_mins",
        rename = "forwarder_apikey_validation_interval"
    )]
    api_key_validation_interval_mins: i64,
}

impl ForwarderConfiguration {
    /// Creates a new `ForwarderConfiguration` from native configuration.
    ///
    /// Fields with no native equivalent (proxy, OPW metrics routing, HTTP protocol, recovery
    /// settings, API-key validation interval) retain their existing defaults. The `min_tls_version`
    /// string mirror is set to keep it consistent with the parsed value.
    pub fn from_native(native: &saluki_component_config::DatadogForwarderConfig) -> Result<Self, GenericError> {
        let parsed_min_tls_version = match native.tls.min_tls_version {
            saluki_component_config::TlsMinimumVersion::Tls1_2 => TlsMinimumVersion::Tls12,
            saluki_component_config::TlsMinimumVersion::Tls1_3 => TlsMinimumVersion::Tls13,
        };
        let min_tls_version = match parsed_min_tls_version {
            TlsMinimumVersion::Tls12 => MIN_TLS_VERSION_TLS12.to_string(),
            TlsMinimumVersion::Tls13 => MIN_TLS_VERSION_TLS13.to_string(),
        };

        Ok(Self {
            endpoint_concurrency: native.endpoint_concurrency,
            endpoint_concurrency_multiplier: native.endpoint_concurrency_multiplier,
            request_timeout_secs: native.request_timeout.as_secs(),
            endpoint_buffer_size: native.endpoint_buffer_size,
            endpoint: EndpointConfiguration::from_native(&native.endpoints),
            retry: RetryConfiguration::from_native(&native.retry),
            proxy: None,
            opw_metrics: OpwMetricsConfiguration::default(),
            http_protocol: ForwarderHttpProtocol::default(),
            connection_reset_interval_secs: native
                .connection_reset_interval
                .map(|interval| interval.as_secs())
                .unwrap_or(0),
            skip_ssl_validation: native.tls.skip_ssl_validation,
            sslkeylogfile: native
                .tls
                .ssl_key_log_file
                .as_ref()
                .map(|path| path.to_string())
                .unwrap_or_default(),
            min_tls_version,
            parsed_min_tls_version,
            allow_arbitrary_tags: native.allow_arbitrary_tags,
            api_key_validation_interval_mins: default_api_key_validation_interval_config_mins(),
        })
    }

    /// Returns the maximum number of concurrent requests for an individual endpoint.
    pub const fn endpoint_concurrency(&self) -> usize {
        let endpoint_concurrency = if self.endpoint_concurrency == 0 {
            1
        } else {
            self.endpoint_concurrency
        };
        let endpoint_concurrency_multiplier = if self.endpoint_concurrency_multiplier == 0 {
            1
        } else {
            self.endpoint_concurrency_multiplier
        };

        endpoint_concurrency.saturating_mul(endpoint_concurrency_multiplier)
    }

    /// Returns the maximum number of idle HTTP connections per host.
    pub const fn max_idle_connections_per_host(&self) -> usize {
        self.endpoint_concurrency_multiplier
    }

    /// Returns the request timeout.
    pub const fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout_secs)
    }

    /// Returns the maximum number of pending requests for an individual endpoint.
    pub const fn endpoint_buffer_size(&self) -> usize {
        self.endpoint_buffer_size
    }

    /// Returns the HTTP protocol selection for outgoing forwarder requests.
    pub fn http_protocol(&self) -> HttpProtocol {
        self.http_protocol.into()
    }

    /// Returns a mutable reference to the endpoint configuration.
    pub fn endpoint_mut(&mut self) -> &mut EndpointConfiguration {
        &mut self.endpoint
    }

    /// Clears the OPW metrics endpoint override.
    pub(crate) fn clear_opw_metrics_endpoint(&mut self) {
        self.opw_metrics = OpwMetricsConfiguration::default();
    }

    /// Builds resolved endpoints with routing metadata.
    ///
    /// The normal primary and OPW metrics primary endpoints share the same dynamic API key source.
    pub(crate) fn build_routable_endpoints(
        &self, configuration: Option<GenericConfiguration>,
    ) -> Result<Vec<RoutableEndpoint>, GenericError> {
        // Label each endpoint so the I/O loop can route metrics to OPW and non-metrics to the normal primary.
        let mut endpoints = Vec::new();
        endpoints.push(RoutableEndpoint::new(
            EndpointRoute::Primary,
            self.endpoint.build_primary_endpoint(configuration.clone())?,
        ));

        if let Some(selected) = self.opw_metrics.selected_endpoint() {
            let trimmed_url = selected.url.trim();
            if trimmed_url.is_empty() {
                warn!(
                    enabled_key = selected.enabled_key,
                    url_key = selected.url_key,
                    "OPW/Vector metrics override is enabled, but no override URL was provided: override will be \
                     disabled. Continuing.",
                );
            } else {
                match self
                    .endpoint
                    .build_primary_endpoint_override(trimmed_url, configuration.clone())
                {
                    Ok(endpoint) => {
                        endpoints.push(RoutableEndpoint::new(EndpointRoute::MetricsPrimary, endpoint));
                    }
                    Err(e) => {
                        warn!(
                            enabled_key = selected.enabled_key,
                            url_key = selected.url_key,
                            url = trimmed_url,
                            error = %e,
                            "Failed to configure OPW/Vector metrics override URL: override will be disabled. Continuing.",
                        );
                    }
                }
            }
        }

        endpoints.extend(
            self.endpoint
                .build_additional_endpoints(configuration.clone())?
                .into_iter()
                .map(|endpoint| RoutableEndpoint::new(EndpointRoute::Additional, endpoint)),
        );

        Ok(endpoints)
    }

    /// Returns a reference to the retry configuration.
    pub const fn retry(&self) -> &RetryConfiguration {
        &self.retry
    }

    /// Returns a reference to the proxy configuration.
    pub const fn proxy(&self) -> &Option<ProxyConfiguration> {
        &self.proxy
    }

    /// Returns the connection reset interval.
    pub const fn connection_reset_interval(&self) -> Duration {
        Duration::from_secs(self.connection_reset_interval_secs)
    }

    /// Returns whether TLS certificate validation is disabled for Datadog intake forwarding.
    pub const fn skip_ssl_validation(&self) -> bool {
        self.skip_ssl_validation
    }

    /// Returns the TLS key log file path, if configured.
    pub fn ssl_key_log_file_path(&self) -> Option<&str> {
        let trimmed = self.sslkeylogfile.trim();
        (!trimmed.is_empty()).then_some(trimmed)
    }

    /// Returns the minimum TLS protocol version for Datadog intake forwarding.
    pub const fn min_tls_version(&self) -> TlsMinimumVersion {
        self.parsed_min_tls_version
    }

    /// Returns whether outbound intake requests should allow arbitrary tag values.
    pub const fn allow_arbitrary_tags(&self) -> bool {
        self.allow_arbitrary_tags
    }

    /// Returns the API key validation interval.
    pub const fn api_key_validation_interval(&self) -> Duration {
        Duration::from_mins(self.api_key_validation_interval_mins as u64)
    }
}
