//! ADP-native runtime configuration.

/// Complete ADP-native runtime configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SalukiConfiguration {
    /// Top-level ADP enablement and pipeline selection.
    pub data_plane: DataPlaneConfiguration,
}

/// Native ADP data-plane runtime decisions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataPlaneConfiguration {
    enabled: bool,
    checks: PipelineConfiguration,
    dogstatsd: PipelineConfiguration,
    otlp: OtlpPipelineConfiguration,
}

impl DataPlaneConfiguration {
    /// Creates native data-plane runtime decisions.
    pub const fn new(
        enabled: bool, checks: PipelineConfiguration, dogstatsd: PipelineConfiguration, otlp: OtlpPipelineConfiguration,
    ) -> Self {
        Self {
            enabled,
            checks,
            dogstatsd,
            otlp,
        }
    }

    /// Returns whether ADP should run.
    pub const fn enabled(&self) -> bool {
        self.enabled
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
        self.checks.enabled() || self.dogstatsd.enabled() || (self.otlp.enabled() && !self.otlp.proxy.enabled())
    }

    /// Returns whether the baseline logs pipeline is needed.
    pub const fn logs_pipeline_required(&self) -> bool {
        self.checks.enabled() || (self.otlp.enabled() && !self.otlp.proxy.enabled())
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
        self.otlp.enabled() && (!self.otlp.proxy.enabled() || !self.otlp.proxy.proxy_traces())
    }
}

/// Native enablement for a simple pipeline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PipelineConfiguration {
    enabled: bool,
}

impl PipelineConfiguration {
    /// Creates pipeline settings.
    pub const fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Returns whether the pipeline is enabled.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
}

/// Native OTLP pipeline settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OtlpPipelineConfiguration {
    enabled: bool,
    proxy: OtlpProxyConfiguration,
}

impl OtlpPipelineConfiguration {
    /// Creates OTLP pipeline settings.
    pub const fn new(enabled: bool, proxy: OtlpProxyConfiguration) -> Self {
        Self { enabled, proxy }
    }

    /// Returns whether OTLP ingest is enabled.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Returns OTLP proxy settings.
    pub const fn proxy(&self) -> &OtlpProxyConfiguration {
        &self.proxy
    }
}

/// Native OTLP proxy settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OtlpProxyConfiguration {
    enabled: bool,
    core_agent_otlp_grpc_endpoint: String,
    proxy_metrics: bool,
    proxy_logs: bool,
    proxy_traces: bool,
}

impl OtlpProxyConfiguration {
    /// Creates OTLP proxy settings.
    pub fn new(
        enabled: bool, core_agent_otlp_grpc_endpoint: String, proxy_metrics: bool, proxy_logs: bool, proxy_traces: bool,
    ) -> Self {
        Self {
            enabled,
            core_agent_otlp_grpc_endpoint,
            proxy_metrics,
            proxy_logs,
            proxy_traces,
        }
    }

    /// Returns whether OTLP proxy mode is enabled.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the Core Agent OTLP gRPC endpoint.
    pub fn core_agent_otlp_grpc_endpoint(&self) -> &str {
        &self.core_agent_otlp_grpc_endpoint
    }

    /// Returns whether metrics are proxied.
    pub const fn proxy_metrics(&self) -> bool {
        self.proxy_metrics
    }

    /// Returns whether logs are proxied.
    pub const fn proxy_logs(&self) -> bool {
        self.proxy_logs
    }

    /// Returns whether traces are proxied.
    pub const fn proxy_traces(&self) -> bool {
        self.proxy_traces
    }
}
