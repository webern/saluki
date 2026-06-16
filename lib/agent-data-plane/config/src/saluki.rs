//! ADP-native runtime configuration.

use std::{path::PathBuf, time::Duration};

use saluki_component_config::{
    AggregateConfiguration, ApmStatsTransformConfiguration, ChecksIpcConfiguration,
    DatadogApmStatsEncoderConfiguration, DatadogEventsEncoderConfiguration, DatadogLogsEncoderConfiguration,
    DatadogMetricsEncoderConfiguration, DatadogServiceChecksEncoderConfiguration, DogStatsDDebugLogConfiguration,
    DogStatsDMapperConfiguration, DogStatsDPostAggregateFilterConfiguration, DogStatsDPrefixFilterConfiguration,
    MultiRegionFailoverConfiguration, OtlpForwarderConfiguration, OtlpPipelineConfiguration, OtlpReceiverConfiguration,
    OtlpSourceConfiguration, OtlpTracesConfiguration, OttlFilterConfiguration, OttlTransformConfiguration,
    PipelineConfiguration, TagFilterlistConfiguration, TraceSamplerConfiguration,
};
use saluki_io::net::ListenAddress;

/// Complete ADP-native runtime configuration.
#[derive(Clone, Debug)]
pub struct SalukiConfiguration {
    /// Top-level ADP enablement and pipeline selection.
    pub data_plane: DataPlaneConfiguration,

    /// Control-plane settings.
    pub control_plane: ControlPlaneConfiguration,

    /// Checks IPC source settings.
    pub checks_ipc: ChecksIpcConfiguration,

    /// OTTL filter settings.
    pub ottl_filter: OttlFilterConfiguration,

    /// OTTL transform settings.
    pub ottl_transform: OttlTransformConfiguration,

    /// Datadog logs encoder settings.
    pub datadog_logs_encoder: DatadogLogsEncoderConfiguration,

    /// Datadog metrics encoder settings.
    pub datadog_metrics_encoder: DatadogMetricsEncoderConfiguration,

    /// Datadog events encoder settings.
    pub datadog_events_encoder: DatadogEventsEncoderConfiguration,

    /// Datadog service-checks encoder settings.
    pub datadog_service_checks_encoder: DatadogServiceChecksEncoderConfiguration,

    /// Datadog APM stats encoder settings.
    pub datadog_apm_stats_encoder: DatadogApmStatsEncoderConfiguration,

    /// APM stats transform settings.
    pub apm_stats_transform: ApmStatsTransformConfiguration,

    /// Trace sampler settings.
    pub trace_sampler: TraceSamplerConfiguration,

    /// Multi-region failover settings.
    pub multi_region_failover: MultiRegionFailoverConfiguration,

    /// DogStatsD listener prefix/filter settings.
    pub dogstatsd_prefix_filter: DogStatsDPrefixFilterConfiguration,

    /// DogStatsD mapper transform settings.
    pub dogstatsd_mapper: DogStatsDMapperConfiguration,

    /// DogStatsD aggregate transform settings.
    pub aggregate: AggregateConfiguration,

    /// DogStatsD debug-log destination settings.
    pub dogstatsd_debug_log: DogStatsDDebugLogConfiguration,

    /// DogStatsD post-aggregate filter settings.
    pub dogstatsd_post_aggregate_filter: DogStatsDPostAggregateFilterConfiguration,

    /// Metric tag filterlist settings.
    pub tag_filterlist: TagFilterlistConfiguration,

    /// OTLP receiver settings.
    pub otlp_receiver: OtlpReceiverConfiguration,

    /// OTLP source settings.
    pub otlp_source: OtlpSourceConfiguration,

    /// OTLP trace processing settings.
    pub otlp_traces: OtlpTracesConfiguration,

    /// OTLP forwarder settings.
    pub otlp_forwarder: OtlpForwarderConfiguration,

    /// Runtime environment provider settings.
    pub environment: EnvironmentConfiguration,
}

/// Native control-plane settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlPlaneConfiguration {
    ipc_cert_file_path: PathBuf,
}

impl ControlPlaneConfiguration {
    /// Creates native control-plane settings.
    pub fn new(ipc_cert_file_path: PathBuf) -> Self {
        Self { ipc_cert_file_path }
    }

    /// Returns the IPC certificate file path used by the privileged API.
    pub fn ipc_cert_file_path(&self) -> &PathBuf {
        &self.ipc_cert_file_path
    }
}

/// Native runtime environment settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnvironmentConfiguration {
    hostname: String,
    host_tags_expected_tags_duration: Duration,
}

impl EnvironmentConfiguration {
    /// Creates native runtime environment settings.
    pub fn new(hostname: String, host_tags_expected_tags_duration: Duration) -> Self {
        Self {
            hostname,
            host_tags_expected_tags_duration,
        }
    }

    /// Returns the configured fixed hostname.
    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    /// Returns how long startup host tags should be added to metrics.
    pub const fn host_tags_expected_tags_duration(&self) -> Duration {
        self.host_tags_expected_tags_duration
    }
}

/// Native ADP data-plane runtime decisions.
#[derive(Clone, Debug)]
pub struct DataPlaneConfiguration {
    enabled: bool,
    standalone_mode: bool,
    api_listen_address: ListenAddress,
    secure_api_listen_address: ListenAddress,
    checks: PipelineConfiguration,
    dogstatsd: PipelineConfiguration,
    otlp: OtlpPipelineConfiguration,
}

impl DataPlaneConfiguration {
    /// Creates native data-plane runtime decisions.
    pub const fn new(
        enabled: bool, standalone_mode: bool, api_listen_address: ListenAddress,
        secure_api_listen_address: ListenAddress, checks: PipelineConfiguration, dogstatsd: PipelineConfiguration,
        otlp: OtlpPipelineConfiguration,
    ) -> Self {
        Self {
            enabled,
            standalone_mode,
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

    /// Returns whether ADP is running without Datadog Agent attachment requirements.
    pub const fn standalone_mode(&self) -> bool {
        self.standalone_mode
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
