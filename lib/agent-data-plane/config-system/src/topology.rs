//! Temporary runtime component configuration adapters.

use saluki_components::{
    forwarders::DatadogForwarderConfiguration, sources::DogStatsDConfiguration,
    transforms::TraceObfuscationConfiguration,
};
use saluki_config::GenericConfiguration;
use saluki_error::{ErrorContext as _, GenericError};

/// Runtime component configuration adapter for topology pieces not yet translated to ADP-native slices.
#[derive(Clone, Debug)]
pub struct RuntimeComponentConfiguration {
    config: GenericConfiguration,
}

impl RuntimeComponentConfiguration {
    pub(crate) const fn new(config: GenericConfiguration) -> Self {
        Self { config }
    }

    /// Builds Datadog forwarder configuration.
    pub fn datadog_forwarder_configuration(&self) -> Result<DatadogForwarderConfiguration, GenericError> {
        DatadogForwarderConfiguration::from_configuration(&self.config)
            .error_context("Failed to configure Datadog forwarder.")
    }

    /// Builds MRF Datadog forwarder configuration.
    pub fn mrf_datadog_forwarder_configuration(
        &self, dd_url: String, api_key: String, api_key_refresh_config_path: &'static str,
    ) -> Result<DatadogForwarderConfiguration, GenericError> {
        DatadogForwarderConfiguration::from_configuration(&self.config)
            .map(|config| {
                config.with_endpoint_override_and_api_key_refresh_config_path(
                    dd_url,
                    api_key,
                    api_key_refresh_config_path,
                )
            })
            .error_context("Failed to configure Multi-Region Failover Datadog forwarder.")
    }

    /// Builds trace obfuscation transform configuration.
    pub fn trace_obfuscation_configuration(&self) -> Result<TraceObfuscationConfiguration, GenericError> {
        TraceObfuscationConfiguration::from_apm_configuration(&self.config)
    }

    /// Builds DogStatsD source configuration.
    pub fn dogstatsd_configuration(&self) -> Result<DogStatsDConfiguration, GenericError> {
        DogStatsDConfiguration::from_configuration(&self.config).error_context("Failed to configure DogStatsD source.")
    }
}
