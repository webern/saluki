//! Multi-region failover configuration.

const MRF_METRICS_ENDPOINT_PREFIX: &str = "https://app.mrf.";

/// Multi-region failover configuration shared by signal-specific pipelines.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MrfConfiguration {
    enabled: bool,
    failover_metrics: bool,
    metric_allowlist: Vec<String>,
    api_key: Option<String>,
    site: Option<String>,
    dd_url: Option<String>,
}

impl MrfConfiguration {
    /// Creates a new `MrfConfiguration` from already-resolved native parts.
    ///
    /// Used by the native (non-raw-map) construction path. `api_key`/`site`/`dd_url` are only needed
    /// to derive a metrics endpoint override; when the forwarder endpoint is already resolved
    /// natively they may be left `None` (the gateway only consults `enabled` / `failover_metrics` /
    /// `metric_allowlist`).
    pub fn new(
        enabled: bool, failover_metrics: bool, metric_allowlist: Vec<String>, api_key: Option<String>,
        site: Option<String>, dd_url: Option<String>,
    ) -> Self {
        Self {
            enabled,
            failover_metrics,
            metric_allowlist,
            api_key,
            site,
            dd_url,
        }
    }

    /// Returns whether multi-region failover is enabled for this process.
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Returns whether metrics forwarding to the failover region is requested by configuration.
    pub const fn is_metrics_forwarding_requested(&self) -> bool {
        self.enabled && self.failover_metrics
    }

    /// Updates whether metrics forwarding to the failover region is enabled.
    pub(crate) const fn set_failover_metrics(&mut self, failover_metrics: bool) {
        self.failover_metrics = failover_metrics;
    }

    /// Updates the metric allowlist.
    pub(crate) fn set_metric_allowlist(&mut self, metric_allowlist: Vec<String>) {
        self.metric_allowlist = metric_allowlist;
    }

    /// Returns the metric allowlist.
    pub fn metric_allowlist(&self) -> &[String] {
        &self.metric_allowlist
    }

    /// Returns the failover-region API key.
    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    /// Returns the failover-region metrics endpoint URL.
    ///
    /// `multi_region_failover.dd_url` takes precedence and is used as provided. When only
    /// `multi_region_failover.site` is configured, the Datadog MRF metrics endpoint is derived from
    /// that site.
    pub fn metrics_endpoint_url(&self) -> Option<String> {
        self.dd_url.clone().or_else(|| {
            self.site
                .as_deref()
                .map(|site| format!("{MRF_METRICS_ENDPOINT_PREFIX}{site}"))
        })
    }

    /// Returns the endpoint and API key override for the failover-region metrics forwarder.
    pub fn metrics_endpoint_override(&self) -> Option<(String, String)> {
        if !self.enabled {
            return None;
        }

        Some((self.metrics_endpoint_url()?, self.api_key.clone()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_endpoint_override_requires_api_key_and_endpoint() {
        let missing_api_key = MrfConfiguration::new(
            true,
            true,
            Vec::new(),
            None,
            Some("datadoghq.eu".to_string()),
            None,
        );
        assert_eq!(missing_api_key.metrics_endpoint_override(), None);

        let missing_endpoint = MrfConfiguration::new(
            true,
            true,
            Vec::new(),
            Some("mrf-api-key".to_string()),
            None,
            None,
        );
        assert_eq!(missing_endpoint.metrics_endpoint_override(), None);

        let ready = MrfConfiguration::new(
            true,
            true,
            Vec::new(),
            Some("mrf-api-key".to_string()),
            None,
            Some("https://mrf.example.com".to_string()),
        );
        assert_eq!(
            ready.metrics_endpoint_override(),
            Some(("https://mrf.example.com".to_string(), "mrf-api-key".to_string()))
        );
    }

    #[test]
    fn metrics_endpoint_override_does_not_require_failover_metrics() {
        let config = MrfConfiguration::new(
            true,
            false,
            Vec::new(),
            Some("mrf-api-key".to_string()),
            None,
            Some("https://mrf.example.com".to_string()),
        );

        assert!(!config.is_metrics_forwarding_requested());
        assert_eq!(
            config.metrics_endpoint_override(),
            Some(("https://mrf.example.com".to_string(), "mrf-api-key".to_string()))
        );
    }

    #[test]
    fn dd_url_takes_precedence_over_site() {
        let config = MrfConfiguration::new(
            false,
            false,
            Vec::new(),
            None,
            Some("datadoghq.eu".to_string()),
            Some("https://custom-mrf.example.com".to_string()),
        );

        assert_eq!(
            config.metrics_endpoint_url().as_deref(),
            Some("https://custom-mrf.example.com")
        );
    }

    #[test]
    fn metrics_endpoint_url_derived_from_site() {
        let config = MrfConfiguration::new(
            true,
            true,
            Vec::new(),
            Some("mrf-api-key".to_string()),
            Some("datadoghq.eu".to_string()),
            None,
        );
        assert_eq!(
            config.metrics_endpoint_url().as_deref(),
            Some("https://app.mrf.datadoghq.eu")
        );
    }
}
