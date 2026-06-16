//! Temporary runtime component configuration adapters.

use std::path::PathBuf;

use saluki_components::{
    config::MrfConfiguration,
    destinations::DogStatsDDebugLogConfiguration,
    encoders::{DatadogApmStatsEncoderConfiguration, DatadogMetricsConfiguration, DatadogTraceConfiguration},
    forwarders::DatadogForwarderConfiguration,
    sources::DogStatsDConfiguration,
    transforms::{
        ApmStatsTransformConfiguration, MrfMetricsGatewayConfiguration, TraceObfuscationConfiguration,
        TraceSamplerConfiguration,
    },
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

    /// Builds Datadog metrics encoder configuration.
    pub fn datadog_metrics_configuration(&self) -> Result<DatadogMetricsConfiguration, GenericError> {
        DatadogMetricsConfiguration::from_configuration(&self.config)
            .error_context("Failed to configure Datadog Metrics encoder.")
    }

    /// Builds MRF configuration.
    pub fn mrf_configuration(&self) -> Result<MrfConfiguration, GenericError> {
        MrfConfiguration::from_configuration(&self.config)
            .error_context("Failed to configure Multi-Region Failover metrics pipeline.")
    }

    /// Builds MRF metrics gateway configuration.
    pub fn mrf_metrics_gateway_configuration(&self, mrf_config: MrfConfiguration) -> MrfMetricsGatewayConfiguration {
        MrfMetricsGatewayConfiguration::new(mrf_config, self.config.clone())
    }

    /// Builds MRF Datadog metrics encoder configuration.
    pub fn mrf_datadog_metrics_configuration(&self) -> Result<DatadogMetricsConfiguration, GenericError> {
        DatadogMetricsConfiguration::from_configuration(&self.config)
            .error_context("Failed to configure Multi-Region Failover Datadog Metrics encoder.")
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

    /// Builds Datadog traces encoder configuration.
    pub fn datadog_trace_configuration(&self) -> Result<DatadogTraceConfiguration, GenericError> {
        DatadogTraceConfiguration::from_configuration(&self.config)
            .error_context("Failed to configure Datadog Traces encoder.")
    }

    /// Builds trace obfuscation transform configuration.
    pub fn trace_obfuscation_configuration(&self) -> Result<TraceObfuscationConfiguration, GenericError> {
        TraceObfuscationConfiguration::from_apm_configuration(&self.config)
    }

    /// Builds trace sampler transform configuration.
    pub fn trace_sampler_configuration(&self) -> Result<TraceSamplerConfiguration, GenericError> {
        TraceSamplerConfiguration::from_configuration(&self.config)
            .error_context("Failed to configure Trace Sampler transform.")
    }

    /// Builds APM stats transform configuration.
    pub fn apm_stats_transform_configuration(&self) -> Result<ApmStatsTransformConfiguration, GenericError> {
        ApmStatsTransformConfiguration::from_configuration(&self.config)
            .error_context("Failed to configure APM Stats transform.")
    }

    /// Builds Datadog APM stats encoder configuration.
    pub fn datadog_apm_stats_encoder_configuration(&self) -> Result<DatadogApmStatsEncoderConfiguration, GenericError> {
        DatadogApmStatsEncoderConfiguration::from_configuration(&self.config)
            .error_context("Failed to configure Datadog APM Stats encoder.")
    }

    /// Builds DogStatsD source configuration.
    pub fn dogstatsd_configuration(&self) -> Result<DogStatsDConfiguration, GenericError> {
        DogStatsDConfiguration::from_configuration(&self.config).error_context("Failed to configure DogStatsD source.")
    }

    /// Builds DogStatsD debug log destination configuration.
    pub fn dogstatsd_debug_log_configuration(
        &self, default_log_file_path: PathBuf,
    ) -> Result<DogStatsDDebugLogConfiguration, GenericError> {
        DogStatsDDebugLogConfiguration::from_configuration(&self.config, default_log_file_path)
            .error_context("Failed to configure DogStatsD debug log destination.")
    }
}
