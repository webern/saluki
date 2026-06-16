//! Bootstrap inputs and local source loading for the configuration system.
//!
//! This is where raw local sources are merged into a `GenericConfiguration` and parsed into the
//! typed [`BootstrapConfiguration`]. The `GenericConfiguration` lives and dies inside this crate; it
//! never crosses the public boundary.

use std::path::PathBuf;

use agent_data_plane_config::{
    BootstrapConfiguration, BootstrapStartupConfiguration, BootstrapTelemetryConfiguration, ConfigStreamAuthority,
    RuntimeConfigAuthority, RuntimeConfigLanguage, RuntimeLoggingConfig,
};
use bytesize::ByteSize;
use saluki_config::{ConfigurationLoader, GenericConfiguration};
use saluki_error::{ErrorContext as _, GenericError};
use saluki_io::net::ListenAddress;

use crate::translate::PipelineGates;

/// Process-local inputs used to start configuration resolution.
///
/// This deliberately does not expose `GenericConfiguration`; it describes only where local sources
/// come from.
#[derive(Clone, Debug)]
pub struct BootstrapInputs {
    /// Path to the local configuration file, if any.
    pub config_file_path: Option<PathBuf>,

    /// Environment variable prefix for local environment sources.
    pub env_var_prefix: &'static str,
}

impl Default for BootstrapInputs {
    fn default() -> Self {
        Self {
            config_file_path: None,
            env_var_prefix: "DD",
        }
    }
}

/// Load and merge raw local sources into a `GenericConfiguration`.
///
/// Migration note: the production loader also applies `KEY_ALIASES` and a `DatadogRemapper`. Those
/// source-precedence details are intentionally omitted in the spike; preserving them is migration
/// work, not an end-state architectural concern.
pub(crate) fn load_local_sources(inputs: &BootstrapInputs) -> Result<GenericConfiguration, GenericError> {
    let mut loader = ConfigurationLoader::default();
    if let Some(path) = &inputs.config_file_path {
        loader = loader
            .from_yaml(path)
            .error_context("Failed to load local configuration file during bootstrap.")?;
    }
    let loader = loader
        .from_environment(inputs.env_var_prefix)
        .error_context("Failed to load local environment configuration during bootstrap.")?;
    Ok(loader.bootstrap_generic())
}

/// Parse the typed [`BootstrapConfiguration`] from merged local sources.
pub(crate) fn parse_bootstrap(config: &GenericConfiguration) -> Result<BootstrapConfiguration, GenericError> {
    let standalone_mode = try_bool(config, "data_plane.standalone_mode")?.unwrap_or(false);

    // End-state authority selection: standalone maps to a local snapshot with no Agent attachment;
    // otherwise the authority is the Datadog Agent config stream. The legacy `remote_agent_enabled`
    // and `use_new_config_stream_endpoint` gates are intentionally not modeled.
    let runtime_config_authority = if standalone_mode {
        RuntimeConfigAuthority::LocalSnapshot(RuntimeConfigLanguage::DatadogAgent)
    } else {
        RuntimeConfigAuthority::ConfigStream(ConfigStreamAuthority::DatadogAgent)
    };

    let secure_api_listen_address = config
        .try_get_typed::<ListenAddress>("data_plane.secure_api_listen_address")
        .error_context("Failed to read `data_plane.secure_api_listen_address`.")?
        .unwrap_or_else(|| ListenAddress::any_tcp(5101));

    Ok(BootstrapConfiguration {
        startup: BootstrapStartupConfiguration {
            runtime_config_authority,
            secure_api_listen_address,
        },
        logging: parse_bootstrap_logging(config)?,
        telemetry: BootstrapTelemetryConfiguration {
            metrics_level: try_string(config, "metrics_level")?,
        },
    })
}

/// Read the pipeline enable/disable gates (ADP control inputs outside the witnessed schema surface).
pub(crate) fn read_pipeline_gates(config: &GenericConfiguration) -> PipelineGates {
    PipelineGates {
        enabled: try_bool(config, "data_plane.enabled").ok().flatten().unwrap_or(false),
        dogstatsd_enabled: try_bool(config, "data_plane.dogstatsd.enabled")
            .ok()
            .flatten()
            .unwrap_or(true),
        checks_enabled: try_bool(config, "data_plane.checks.enabled")
            .ok()
            .flatten()
            .unwrap_or(false),
        otlp_enabled: try_bool(config, "data_plane.otlp.enabled")
            .ok()
            .flatten()
            .unwrap_or(false),
    }
}

/// Read the pipeline gates from an authoritative snapshot value (connected path).
pub(crate) fn read_pipeline_gates_value(snapshot: &serde_json::Value) -> PipelineGates {
    let bool_at = |path: &str, default: bool| -> bool {
        let mut cur = snapshot;
        for part in path.split('.') {
            match cur.get(part) {
                Some(v) => cur = v,
                None => return default,
            }
        }
        cur.as_bool().unwrap_or(default)
    };

    PipelineGates {
        enabled: bool_at("data_plane.enabled", false),
        dogstatsd_enabled: bool_at("data_plane.dogstatsd.enabled", true),
        checks_enabled: bool_at("data_plane.checks.enabled", false),
        otlp_enabled: bool_at("data_plane.otlp.enabled", false),
    }
}

/// Read the OTLP proxy control inputs (`data_plane.otlp.proxy.*`), which are ADP control keys
/// outside the witnessed schema surface. Returns `None` when proxy mode is disabled.
pub(crate) fn read_otlp_proxy(config: &GenericConfiguration) -> Option<saluki_component_config::OtlpProxyConfig> {
    let enabled = config
        .try_get_typed::<bool>("data_plane.otlp.proxy.enabled")
        .ok()
        .flatten()
        .unwrap_or(false);
    if !enabled {
        return None;
    }

    let grpc_endpoint = config
        .try_get_typed::<String>("data_plane.otlp.proxy.receiver.protocols.grpc.endpoint")
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "http://localhost:4319".to_string());
    let traces_internal_port = config
        .try_get_typed::<i64>("otlp_config.traces.internal_port")
        .ok()
        .flatten()
        .unwrap_or(0)
        .clamp(0, u16::MAX as i64) as u16;

    let flag = |key: &str| config.try_get_typed::<bool>(key).ok().flatten().unwrap_or(true);

    Some(saluki_component_config::OtlpProxyConfig {
        core_agent_otlp_grpc_endpoint: stringtheory::MetaString::from(grpc_endpoint),
        proxy_metrics: flag("data_plane.otlp.proxy.metrics.enabled"),
        proxy_logs: flag("data_plane.otlp.proxy.logs.enabled"),
        proxy_traces: flag("data_plane.otlp.proxy.traces.enabled"),
        core_agent_traces_internal_port: traces_internal_port,
    })
}

/// Read the workload-metadata tuning knobs (environment provider control inputs outside the
/// witnessed schema surface).
pub(crate) fn read_workload_config(config: &GenericConfiguration) -> agent_data_plane_config::WorkloadPrivateConfig {
    let path = |key: &str| {
        config
            .try_get_typed::<std::path::PathBuf>(key)
            .ok()
            .flatten()
            .filter(|p| !p.as_os_str().is_empty())
    };
    agent_data_plane_config::WorkloadPrivateConfig {
        string_interner_size_bytes: config
            .try_get_typed::<usize>("remote_agent_string_interner_size_bytes")
            .ok()
            .flatten(),
        containerd_socket_path: path("cri_socket_path"),
        container_proc_root: path("container_proc_root"),
        container_cgroup_root: path("container_cgroup_root"),
    }
}

/// Read the ADP memory-bounds control inputs (outside the witnessed schema surface).
pub(crate) fn read_memory_config(config: &GenericConfiguration) -> agent_data_plane_config::MemoryConfig {
    agent_data_plane_config::MemoryConfig {
        memory_limit_bytes: config
            .try_get_typed::<ByteSize>("memory_limit")
            .ok()
            .flatten()
            .map(|b| b.as_u64()),
        slop_factor: config.try_get_typed::<f64>("memory_slop_factor").ok().flatten(),
        enable_global_limiter: config.try_get_typed::<bool>("enable_global_limiter").ok().flatten(),
    }
}

fn parse_bootstrap_logging(config: &GenericConfiguration) -> Result<RuntimeLoggingConfig, GenericError> {
    let mut logging = RuntimeLoggingConfig::default();
    logging.log_level = try_string(config, "log_level")?;
    if let Some(v) = try_bool(config, "log_format_json")? {
        logging.log_format_json = v;
    }
    if let Some(v) = try_bool(config, "log_format_rfc3339")? {
        logging.log_format_rfc3339 = v;
    }
    if let Some(v) = try_bool(config, "log_to_console")? {
        logging.log_to_console = v;
    }
    if let Some(v) = try_bool(config, "log_to_syslog")? {
        logging.log_to_syslog = v;
    }
    if let Some(v) = try_bool(config, "syslog_rfc")? {
        logging.syslog_rfc = v;
    }
    if let Some(v) = try_string(config, "syslog_uri")? {
        logging.syslog_uri = v;
    }
    if let Some(v) = try_string(config, "data_plane.log_file")? {
        logging.log_file = v;
    }
    if let Some(v) = config
        .try_get_typed::<ByteSize>("log_file_max_size")
        .error_context("Failed to read `log_file_max_size`.")?
    {
        logging.log_file_max_size = v;
    }
    if let Some(v) = config
        .try_get_typed::<usize>("log_file_max_rolls")
        .error_context("Failed to read `log_file_max_rolls`.")?
    {
        logging.log_file_max_rolls = v;
    }
    Ok(logging)
}

fn try_bool(config: &GenericConfiguration, key: &str) -> Result<Option<bool>, GenericError> {
    config
        .try_get_typed::<bool>(key)
        .with_error_context(|| format!("Failed to read `{key}`."))
}

fn try_string(config: &GenericConfiguration, key: &str) -> Result<Option<String>, GenericError> {
    config
        .try_get_typed::<String>(key)
        .with_error_context(|| format!("Failed to read `{key}`."))
}
