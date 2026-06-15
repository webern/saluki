//! ADP-native runtime configuration.

use saluki_component_config::{OtlpPipelineConfiguration, PipelineConfiguration};
use saluki_io::net::ListenAddress;

/// Complete ADP-native runtime configuration.
#[derive(Clone, Debug)]
pub struct SalukiConfiguration {
    /// Top-level ADP enablement and pipeline selection.
    pub data_plane: DataPlaneConfiguration,
}

/// Native ADP data-plane runtime decisions.
#[derive(Clone, Debug)]
pub struct DataPlaneConfiguration {
    enabled: bool,
    api_listen_address: ListenAddress,
    secure_api_listen_address: ListenAddress,
    checks: PipelineConfiguration,
    dogstatsd: PipelineConfiguration,
    otlp: OtlpPipelineConfiguration,
}

impl DataPlaneConfiguration {
    /// Creates native data-plane runtime decisions.
    pub const fn new(
        enabled: bool, api_listen_address: ListenAddress, secure_api_listen_address: ListenAddress,
        checks: PipelineConfiguration, dogstatsd: PipelineConfiguration, otlp: OtlpPipelineConfiguration,
    ) -> Self {
        Self {
            enabled,
            api_listen_address,
            secure_api_listen_address,
            checks,
            dogstatsd,
            otlp,
        }
    }

    /// Returns whether ADP should run.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the unprivileged API listen address.
    pub const fn api_listen_address(&self) -> &ListenAddress {
        &self.api_listen_address
    }

    /// Returns the privileged API listen address.
    pub const fn secure_api_listen_address(&self) -> &ListenAddress {
        &self.secure_api_listen_address
    }

    /// Returns the checks pipeline settings.
    pub const fn checks(&self) -> &PipelineConfiguration {
        &self.checks
    }

    /// Returns the DogStatsD pipeline settings.
    pub const fn dogstatsd(&self) -> &PipelineConfiguration {
        &self.dogstatsd
    }

    /// Returns the OTLP pipeline settings.
    pub const fn otlp(&self) -> &OtlpPipelineConfiguration {
        &self.otlp
    }

    /// Returns whether any data pipeline is enabled.
    pub const fn data_pipelines_enabled(&self) -> bool {
        self.checks.enabled() || self.dogstatsd.enabled() || self.otlp.enabled()
    }

    /// Returns whether the Datadog forwarder is needed.
    pub const fn requires_datadog_forwarder(&self) -> bool {
        self.metrics_pipeline_required()
            || self.logs_pipeline_required()
            || self.events_pipeline_required()
            || self.service_checks_pipeline_required()
            || self.traces_pipeline_required()
    }

    /// Returns whether the baseline metrics pipeline is needed.
    pub const fn metrics_pipeline_required(&self) -> bool {
        self.checks.enabled() || self.dogstatsd.enabled() || (self.otlp.enabled() && !self.otlp.proxy().enabled())
    }

    /// Returns whether the baseline logs pipeline is needed.
    pub const fn logs_pipeline_required(&self) -> bool {
        self.checks.enabled() || (self.otlp.enabled() && !self.otlp.proxy().enabled())
    }

    /// Returns whether the baseline events pipeline is needed.
    pub const fn events_pipeline_required(&self) -> bool {
        self.checks.enabled() || self.dogstatsd.enabled()
    }

    /// Returns whether the baseline service-checks pipeline is needed.
    pub const fn service_checks_pipeline_required(&self) -> bool {
        self.checks.enabled() || self.dogstatsd.enabled()
    }

    /// Returns whether the baseline traces pipeline is needed.
    pub const fn traces_pipeline_required(&self) -> bool {
        self.otlp.enabled() && (!self.otlp.proxy().enabled() || !self.otlp.proxy().proxy_traces())
    }
}
