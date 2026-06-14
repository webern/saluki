use bytesize::ByteSize;
use datadog_agent_commons::ipc::config::RemoteAgentClientConfiguration;
use saluki_common::deser::PermissiveBool;
use saluki_config::{ConfigurationLoader, GenericConfiguration};
use saluki_error::{ErrorContext as _, GenericError};
use saluki_io::net::ListenAddress;
use serde::Deserialize;
use serde_with::serde_as;

/// General data plane configuration.
#[derive(Clone, Debug)]
pub struct DataPlaneConfiguration {
    enabled: bool,
    standalone_mode: bool,
    use_new_config_stream_endpoint: bool,
    remote_agent_enabled: bool,
    api_listen_address: ListenAddress,
    secure_api_listen_address: ListenAddress,
    checks: DataPlaneChecksConfiguration,
    dogstatsd: DataPlaneDogStatsDConfiguration,
    otlp: DataPlaneOtlpConfiguration,
}

impl DataPlaneConfiguration {
    /// Creates a new `DataPlaneConfiguration` instance from the given configuration.
    ///
    /// # Errors
    ///
    /// If the configuration can't be deserialized, an error is returned.
    pub fn from_configuration(config: &GenericConfiguration) -> Result<Self, GenericError> {
        // TODO: We're explicitly querying each individual field from the configuration because if we don't, then our
        // environment variable overrides end up requiring double underscores to indicate nesting (i.e. we have to do
        // `DD_DATA_PLANE__OTLP__ENABLED` instead of just `DD_DATA_PLANE_OTLP_ENABLED`). I find this personally ugly,
        // and it would also fly in the face of environment variable naming conventions for existing Agent settings.
        //
        // In the future, we plan on updating `saluki-config` to allow us to support both deserializing from "native"
        // nested data like JSON/YAML as well as with the idiomatically-named environment variables.
        Ok(Self {
            enabled: config.try_get_typed("data_plane.enabled")?.unwrap_or(false),
            standalone_mode: config.try_get_typed("data_plane.standalone_mode")?.unwrap_or(false),
            use_new_config_stream_endpoint: config
                .try_get_typed("data_plane.use_new_config_stream_endpoint")?
                .unwrap_or(true),
            remote_agent_enabled: config.try_get_typed("data_plane.remote_agent_enabled")?.unwrap_or(true),
            api_listen_address: config
                .try_get_typed("data_plane.api_listen_address")?
                .unwrap_or_else(|| ListenAddress::any_tcp(5100)),
            secure_api_listen_address: config
                .try_get_typed("data_plane.secure_api_listen_address")?
                .unwrap_or_else(|| ListenAddress::any_tcp(5101)),
            checks: DataPlaneChecksConfiguration::from_configuration(config)?,
            dogstatsd: DataPlaneDogStatsDConfiguration::from_configuration(config)?,
            otlp: DataPlaneOtlpConfiguration::from_configuration(config)?,
        })
    }

    /// Returns `true` if the data plane is enabled.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Returns `true` if the data plane is running in standalone mode.
    pub const fn standalone_mode(&self) -> bool {
        self.standalone_mode
    }

    /// Returns `true` if the new config stream endpoint should be used.
    pub const fn use_new_config_stream_endpoint(&self) -> bool {
        self.use_new_config_stream_endpoint
    }

    /// Returns `true` if the data plane should register as a remote agent.
    pub const fn remote_agent_enabled(&self) -> bool {
        self.remote_agent_enabled
    }

    /// Returns a reference to the API listen address
    ///
    /// This is also referred to as the "unprivileged" API.
    pub const fn api_listen_address(&self) -> &ListenAddress {
        &self.api_listen_address
    }

    /// Returns a reference to the secure API listen address.
    ///
    /// This is also referred to as the "privileged" API.
    pub const fn secure_api_listen_address(&self) -> &ListenAddress {
        &self.secure_api_listen_address
    }

    /// Returns a reference to the Checks-specific data plane configuration.
    pub const fn checks(&self) -> &DataPlaneChecksConfiguration {
        &self.checks
    }

    /// Returns a reference to the DogStatsD-specific data plane configuration.
    pub const fn dogstatsd(&self) -> &DataPlaneDogStatsDConfiguration {
        &self.dogstatsd
    }

    /// Returns a reference to the OTLP-specific data plane configuration.
    pub const fn otlp(&self) -> &DataPlaneOtlpConfiguration {
        &self.otlp
    }

    /// Returns `true` if any data pipelines are enabled.
    pub const fn data_pipelines_enabled(&self) -> bool {
        self.checks().enabled() || self.dogstatsd().enabled() || self.otlp().enabled()
    }

    /// Returns `true` if the metrics pipeline is required.
    ///
    /// This indicates that the "baseline" metrics pipeline (aggregation, enrichment, encoding, forwarding) is required
    /// by higher-level data pipelines, such as DogStatsD.
    pub const fn metrics_pipeline_required(&self) -> bool {
        // We consider the metrics pipeline to be enabled if:
        // - Checks is enabled
        // - DogStatsD is enabled
        // - OTLP is enabled and not in proxy mode
        self.checks().enabled()
            || self.dogstatsd().enabled()
            || (self.otlp().enabled() && !self.otlp().proxy().enabled())
    }

    /// Returns `true` if the logs pipeline is required.
    ///
    /// This indicates that the "baseline" logs pipeline (encoding, forwarding) is required by higher-level data
    /// pipelines, such as Checks or OTLP.
    pub const fn logs_pipeline_required(&self) -> bool {
        // We consider the logs pipeline to be enabled if:
        // - Checks is enabled
        // - OTLP is enabled and not in proxy mode
        self.checks().enabled() || (self.otlp().enabled() && !self.otlp().proxy().enabled())
    }

    /// Returns `true` if the events pipeline is required.
    ///
    /// This indicates that the "baseline" events pipeline (encoding, forwarding) is required by higher-level data
    /// pipelines, such as Checks or DogStatsD.
    pub const fn events_pipeline_required(&self) -> bool {
        self.checks().enabled() || self.dogstatsd().enabled()
    }

    /// Returns `true` if the service checks pipeline is required.
    ///
    /// This indicates that the "baseline" service checks pipeline (encoding, forwarding) is required by higher-level
    /// data pipelines, such as Checks or DogStatsD.
    pub const fn service_checks_pipeline_required(&self) -> bool {
        self.checks().enabled() || self.dogstatsd().enabled()
    }

    /// Returns `true` if the traces pipeline is required.
    ///
    /// This indicates that the "baseline" traces pipeline (encoding, forwarding) is required by higher-level data
    /// pipelines, such as OTLP.
    pub const fn traces_pipeline_required(&self) -> bool {
        // We consider the traces pipeline to be enabled if:
        // - OTLP is enabled and not in proxy mode or proxy mode is enabled and proxy traces are disabled
        self.otlp().enabled() && (!self.otlp().proxy().enabled() || !self.otlp().proxy().proxy_traces())
    }
}

/// Checks-specific data plane configuration.
#[derive(Clone, Debug)]
pub struct DataPlaneChecksConfiguration {
    /// Whether Checks is enabled.
    ///
    /// When disabled, Checks won't be started.
    ///
    /// Defaults to `false`.
    enabled: bool,
}

impl DataPlaneChecksConfiguration {
    fn from_configuration(config: &GenericConfiguration) -> Result<Self, GenericError> {
        Ok(Self {
            enabled: config.try_get_typed("data_plane.checks.enabled")?.unwrap_or(false),
        })
    }

    /// Returns `true` if Checks is enabled.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
}

/// DogStatsD-specific data plane configuration.
#[derive(Clone, Debug)]
pub struct DataPlaneDogStatsDConfiguration {
    /// Whether DogStatsD is enabled.
    ///
    /// When disabled, DogStatsD won't be started.
    ///
    /// Defaults to `true`.
    enabled: bool,
}

impl DataPlaneDogStatsDConfiguration {
    // We intentionally do NOT read the Core Agent's `use_dogstatsd` key here. The Core Agent is the
    // sole authority on whether ADP should run DogStatsD: it evaluates `use_dogstatsd` (along with
    // other signals) and sets `data_plane.dogstatsd.enabled` on our behalf. Reading both would risk
    // ADP and the Core Agent disagreeing. See `docs/agent-data-plane/configuration/dogstatsd.md`.
    fn from_configuration(config: &GenericConfiguration) -> Result<Self, GenericError> {
        Ok(Self {
            enabled: config.try_get_typed("data_plane.dogstatsd.enabled")?.unwrap_or(true),
        })
    }

    /// Returns `true` if DogStatsD is enabled.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
}

/// OTLP-specific data plane configuration.
#[derive(Clone, Debug)]
pub struct DataPlaneOtlpConfiguration {
    enabled: bool,
    proxy: DataPlaneOtlpProxyConfiguration,
}

impl DataPlaneOtlpConfiguration {
    fn from_configuration(config: &GenericConfiguration) -> Result<Self, GenericError> {
        Ok(Self {
            enabled: config.try_get_typed("data_plane.otlp.enabled")?.unwrap_or(false),
            proxy: DataPlaneOtlpProxyConfiguration::from_configuration(config)?,
        })
    }

    /// Returns `true` if the OTLP pipeline is enabled.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Returns a reference to the OTLP proxying configuration.
    pub const fn proxy(&self) -> &DataPlaneOtlpProxyConfiguration {
        &self.proxy
    }
}

/// OTLP proxying configuration.
///
/// In proxy mode, ADP takes over the normal "OTLP Ingest" endpoints that the Core Agent would typically listen on,
/// so the Core Agent must be configured to listen on a different, separate port than it usually would so that ADP
/// can proxy to it.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct DataPlaneOtlpProxyConfiguration {
    /// Whether or not to proxy all signals to the Agent.
    ///
    /// When enabled, OTLP signals which aren't supported by ADP will be proxied to the Agent. Depending on the signal
    /// type, they may be proxied to either the Core Agent or Trace Agent.
    ///
    /// Defaults to `true`.
    enabled: bool,

    /// OTLP gRPC endpoint on the Core Agent to proxy signals to.
    ///
    /// Defaults to `http://localhost:4319`.
    core_agent_otlp_grpc_endpoint: String,

    /// Whether or not to proxy traces to the Core Agent.
    ///
    /// Defaults to `true`.
    proxy_traces: bool,

    /// Whether or not to proxy metrics to the Core Agent.
    ///
    /// Defaults to `true`.
    proxy_metrics: bool,

    /// Whether or not to proxy logs to the Core Agent.
    ///
    /// Defaults to `true`.
    proxy_logs: bool,
}

impl DataPlaneOtlpProxyConfiguration {
    fn from_configuration(config: &GenericConfiguration) -> Result<Self, GenericError> {
        let enabled = config.try_get_typed("data_plane.otlp.proxy.enabled")?.unwrap_or(false);
        let core_agent_otlp_grpc_endpoint = config
            .try_get_typed("data_plane.otlp.proxy.receiver.protocols.grpc.endpoint")?
            .unwrap_or("http://localhost:4319".to_string());
        let proxy_traces = config
            .try_get_typed("data_plane.otlp.proxy.traces.enabled")?
            .unwrap_or(true);
        let proxy_metrics = config
            .try_get_typed("data_plane.otlp.proxy.metrics.enabled")?
            .unwrap_or(true);
        let proxy_logs = config
            .try_get_typed("data_plane.otlp.proxy.logs.enabled")?
            .unwrap_or(true);

        Ok(Self {
            enabled,
            core_agent_otlp_grpc_endpoint,
            proxy_traces,
            proxy_metrics,
            proxy_logs,
        })
    }

    /// Returns `true` if the OTLP proxy is enabled.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the OTLP gRPC endpoint on the Core Agent to proxy signals to.
    pub fn core_agent_otlp_grpc_endpoint(&self) -> &str {
        &self.core_agent_otlp_grpc_endpoint
    }

    /// Returns `true` if the OTLP traces should be proxied to the Core Agent.
    pub const fn proxy_traces(&self) -> bool {
        self.proxy_traces
    }

    /// Returns `true` if the OTLP metrics should be proxied to the Core Agent.
    pub const fn proxy_metrics(&self) -> bool {
        self.proxy_metrics
    }

    /// Returns `true` if the OTLP logs should be proxied to the Core Agent.
    pub const fn proxy_logs(&self) -> bool {
        self.proxy_logs
    }
}

// ---------------------------------------------------------------------------
// BootstrapConfiguration
// ---------------------------------------------------------------------------

/// Pre-authority bootstrap configuration for ADP startup.
///
/// Every key here is read before `dynamic_config.ready().await` in `cli/run.rs`, the point where
/// the first Agent config map arrives. Bootstrap is a lifecycle phase, not a trust domain: it spans
/// Datadog-schema keys (logging, IPC/auth/TLS) and Saluki-domain keys (startup gates, ADP log
/// file, metrics level), all arriving from the single local bootstrap loader.
///
/// Saluki-domain bootstrap keys (`standalone_mode`, `agent_ipc_endpoint`, the startup gates, etc.)
/// currently arrive via `DD_*` / `DD_DATA_PLANE_*`. That is a known temporary wart: the intended
/// end-state is to source them from `SALUKI_*` and reject the `DD_*` form, but that migration is
/// deferred. See the "source authority for Saluki-domain bootstrap keys" TODO in `design/mapless.md`.
#[allow(dead_code)]
pub struct BootstrapConfiguration {
    // Datadog-domain: logging keys (shared with Core Agent schema)
    pub(crate) log_level: Option<String>,
    pub(crate) log_format_json: Option<bool>,
    pub(crate) log_format_rfc3339: Option<bool>,
    pub(crate) log_to_console: Option<bool>,
    pub(crate) log_to_syslog: Option<bool>,
    pub(crate) syslog_rfc: Option<bool>,
    pub(crate) syslog_uri: Option<String>,
    pub(crate) log_file_max_size: Option<ByteSize>,
    pub(crate) log_file_max_rolls: Option<usize>,
    pub(crate) disable_file_logging: bool,
    // Saluki-domain: ADP log file (never shares with Core Agent log_file)
    pub(crate) data_plane_log_file: Option<String>,
    // Saluki-domain: metrics bootstrap level
    pub(crate) metrics_level: Option<String>,
    // Saluki-domain startup gates (via DD_DATA_PLANE_* until the sourcing migration)
    pub(crate) standalone_mode: bool,
    pub(crate) remote_agent_enabled: bool,
    pub(crate) use_new_config_stream_endpoint: bool,
    // Saluki-domain: RAR rendezvous address
    pub(crate) secure_api_listen_address: ListenAddress,
    // IPC/auth/TLS — Datadog-domain: cmd_port, auth_token_file_path, ipc_cert_file_path, vsock_addr;
    //                 Saluki-domain:  agent_ipc_endpoint, agent_ipc_grpc_max_message_size,
    //                                 connect_retry_attempts, connect_retry_backoff
    pub(crate) ipc_client: RemoteAgentClientConfiguration,
}

/// A helper wrapper to parse permissive booleans (accepts "true"/"false", "1"/"0", etc.) from config.
#[serde_as]
#[derive(Deserialize)]
struct PermissiveBoolValue(#[serde_as(as = "PermissiveBool")] bool);

fn read_permissive_bool(config: &GenericConfiguration, key: &str) -> Result<Option<bool>, GenericError> {
    Ok(config
        .try_get_typed::<PermissiveBoolValue>(key)
        .with_error_context(|| format!("Failed to read `{}`.", key))?
        .map(|v| v.0))
}

impl BootstrapConfiguration {
    /// Builds a `BootstrapConfiguration` from the given generic configuration.
    ///
    /// Source precedence is inherited from the loader used to build `config`:
    /// `from_yaml` < `DatadogRemapper` < `from_environment(DD_)` with `KEY_ALIASES`.
    ///
    /// # Errors
    ///
    /// If any key is present but cannot be parsed into the expected type.
    pub fn from_configuration(config: &GenericConfiguration) -> Result<Self, GenericError> {
        Ok(Self {
            log_level: config
                .try_get_typed::<String>("log_level")
                .error_context("Failed to read `log_level`.")?,
            log_format_json: read_permissive_bool(config, "log_format_json")?,
            log_format_rfc3339: read_permissive_bool(config, "log_format_rfc3339")?,
            log_to_console: read_permissive_bool(config, "log_to_console")?,
            log_to_syslog: read_permissive_bool(config, "log_to_syslog")?,
            syslog_rfc: read_permissive_bool(config, "syslog_rfc")?,
            syslog_uri: config
                .try_get_typed::<String>("syslog_uri")
                .error_context("Failed to read `syslog_uri`.")?,
            log_file_max_size: config
                .try_get_typed::<ByteSize>("log_file_max_size")
                .error_context("Failed to read `log_file_max_size`.")?,
            log_file_max_rolls: config
                .try_get_typed::<usize>("log_file_max_rolls")
                .error_context("Failed to read `log_file_max_rolls`.")?,
            disable_file_logging: read_permissive_bool(config, "disable_file_logging")?.unwrap_or(false),
            data_plane_log_file: config
                .try_get_typed::<String>("data_plane.log_file")
                .error_context("Failed to read `data_plane.log_file`.")?,
            metrics_level: config
                .try_get_typed::<String>("metrics_level")
                .error_context("Failed to read `metrics_level`.")?,
            standalone_mode: config
                .try_get_typed("data_plane.standalone_mode")
                .error_context("Failed to read `data_plane.standalone_mode`.")?
                .unwrap_or(false),
            remote_agent_enabled: config
                .try_get_typed("data_plane.remote_agent_enabled")
                .error_context("Failed to read `data_plane.remote_agent_enabled`.")?
                .unwrap_or(true),
            use_new_config_stream_endpoint: config
                .try_get_typed("data_plane.use_new_config_stream_endpoint")
                .error_context("Failed to read `data_plane.use_new_config_stream_endpoint`.")?
                .unwrap_or(true),
            secure_api_listen_address: config
                .try_get_typed("data_plane.secure_api_listen_address")
                .error_context("Failed to read `data_plane.secure_api_listen_address`.")?
                .unwrap_or_else(|| ListenAddress::any_tcp(5101)),
            ipc_client: RemoteAgentClientConfiguration::from_configuration(config)
                .error_context("Failed to read IPC client configuration from bootstrap.")?,
        })
    }

    /// Returns the configured metrics level string, if set.
    pub fn metrics_level(&self) -> Option<&str> {
        self.metrics_level.as_deref()
    }
}

// ---------------------------------------------------------------------------
// SalukiPrivateConfiguration
// ---------------------------------------------------------------------------

/// Saluki-private runtime configuration loader (scaffolding — no component cutover in PR 2).
///
/// Loads ADP-private configuration from `SALUKI_*` environment variables and an optional
/// `saluki.yaml` file. Deliberately excludes all Datadog sources (`DD_*`, `datadog.yaml`,
/// RAR/config-stream) and bootstrap-only keys.
///
/// The runtime key set is the entries from `SALUKI_KEYS` minus the bootstrap-only Saluki-domain
/// keys: `data_plane.remote_agent_enabled`, `data_plane.use_new_config_stream_endpoint`,
/// `data_plane.standalone_mode`, `data_plane.secure_api_listen_address`, `metrics_level`,
/// `data_plane.log_file`, `agent_ipc_endpoint`, `agent_ipc_grpc_max_message_size`,
/// `connect_retry_attempts`, and `connect_retry_backoff`.
///
/// No component is cut over to this config in PR 2. Component migrations happen in PRs 4-9.
/// Source-boundary tests verify that Datadog sources cannot reach this config surface.
#[allow(dead_code)]
pub struct SalukiPrivateConfiguration {
    inner: GenericConfiguration,
}

#[allow(dead_code)]
impl SalukiPrivateConfiguration {
    /// Loads `SalukiPrivateConfiguration` from `SALUKI_*` env vars and optional `saluki.yaml`.
    ///
    /// `saluki_yaml_path`, if provided and the file exists, is loaded first. `SALUKI_*` env vars
    /// take precedence over the file. No `DD_*` provider is added: this is the hard source
    /// boundary separating Saluki-private config from Datadog config.
    ///
    /// # Errors
    ///
    /// If `SALUKI` is an empty prefix (should not happen) or if the loader fails.
    pub fn load(saluki_yaml_path: Option<&std::path::Path>) -> Result<Self, GenericError> {
        let mut loader = ConfigurationLoader::default();

        if let Some(path) = saluki_yaml_path {
            loader = loader.try_from_yaml(path);
        }

        loader = loader
            .from_environment("SALUKI")
            .error_context("Failed to add SALUKI_* environment provider.")?;

        Ok(Self {
            inner: loader.bootstrap_generic(),
        })
    }

    /// Returns the inner `GenericConfiguration` for key access.
    ///
    /// Components will use typed accessors in later PRs; this exists for source-boundary tests.
    pub fn inner(&self) -> &GenericConfiguration {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use saluki_config::ConfigurationLoader;
    use serde_json::json;

    use super::*;

    // ADP ignores `use_dogstatsd`. The Core Agent evaluates that key and delivers the resolved
    // decision to ADP by setting `data_plane.dogstatsd.enabled` via the config stream.

    #[tokio::test]
    async fn default_enables_dogstatsd() {
        let (config, _) =
            ConfigurationLoader::for_tests(Some(json!({ "data_plane": { "enabled": true } })), None, false).await;

        let dp = DataPlaneConfiguration::from_configuration(&config).expect("parse config");
        assert!(dp.enabled());
        assert!(dp.dogstatsd().enabled());
    }

    #[tokio::test]
    async fn use_dogstatsd_false_does_not_disable_dogstatsd_by_default() {
        let (config, _) = ConfigurationLoader::for_tests(
            Some(json!({ "use_dogstatsd": false, "data_plane": { "enabled": true } })),
            None,
            false,
        )
        .await;

        let dp = DataPlaneConfiguration::from_configuration(&config).expect("parse config");
        assert!(dp.enabled());
        assert!(dp.dogstatsd().enabled());
    }

    #[tokio::test]
    async fn explicit_false_disables_dogstatsd() {
        let (config, _) = ConfigurationLoader::for_tests(
            Some(json!({ "data_plane": { "enabled": true, "dogstatsd": { "enabled": false } } })),
            None,
            false,
        )
        .await;

        let dp = DataPlaneConfiguration::from_configuration(&config).expect("parse config");
        assert!(dp.enabled());
        assert!(!dp.dogstatsd().enabled());
    }

    #[tokio::test]
    async fn use_dogstatsd_true_does_not_override_explicit_false() {
        // `use_dogstatsd=true` must not enable DSD when `data_plane.dogstatsd.enabled=false` is
        // set explicitly. ADP reads only its own key.
        let (config, _) = ConfigurationLoader::for_tests(
            Some(json!({
                "use_dogstatsd": true,
                "data_plane": { "enabled": true, "dogstatsd": { "enabled": false } },
            })),
            None,
            false,
        )
        .await;

        let dp = DataPlaneConfiguration::from_configuration(&config).expect("parse config");
        assert!(dp.enabled());
        assert!(!dp.dogstatsd().enabled());
    }

    #[tokio::test]
    async fn use_dogstatsd_false_does_not_disable_dogstatsd_when_explicitly_enabled() {
        // `use_dogstatsd=false` must not disable DSD when `data_plane.dogstatsd.enabled=true` is
        // set explicitly. The Core Agent communicates its resolved decision via that key.
        let (config, _) = ConfigurationLoader::for_tests(
            Some(json!({
                "use_dogstatsd": false,
                "data_plane": { "enabled": true, "dogstatsd": { "enabled": true } },
            })),
            None,
            false,
        )
        .await;

        let dp = DataPlaneConfiguration::from_configuration(&config).expect("parse config");
        assert!(dp.enabled());
        assert!(dp.dogstatsd().enabled());
    }

    // BootstrapConfiguration tests

    #[tokio::test]
    async fn bootstrap_defaults_when_empty_config() {
        let (config, _) = ConfigurationLoader::for_tests(None, None, false).await;
        let bc = BootstrapConfiguration::from_configuration(&config).expect("parse bootstrap config");

        assert!(bc.log_level.is_none());
        assert!(!bc.standalone_mode);
        assert!(bc.remote_agent_enabled);
        assert!(bc.use_new_config_stream_endpoint);
    }

    #[tokio::test]
    async fn bootstrap_reads_standalone_mode_from_config() {
        let (config, _) =
            ConfigurationLoader::for_tests(Some(json!({ "data_plane": { "standalone_mode": true } })), None, false)
                .await;
        let bc = BootstrapConfiguration::from_configuration(&config).expect("parse bootstrap config");
        assert!(bc.standalone_mode);
    }

    #[tokio::test]
    async fn bootstrap_reads_log_level_from_config() {
        let (config, _) = ConfigurationLoader::for_tests(Some(json!({ "log_level": "debug" })), None, false).await;
        let bc = BootstrapConfiguration::from_configuration(&config).expect("parse bootstrap config");
        assert_eq!(bc.log_level.as_deref(), Some("debug"));
    }

    #[tokio::test]
    async fn bootstrap_reads_metrics_level_from_config() {
        let (config, _) = ConfigurationLoader::for_tests(Some(json!({ "metrics_level": "debug" })), None, false).await;
        let bc = BootstrapConfiguration::from_configuration(&config).expect("parse bootstrap config");
        assert_eq!(bc.metrics_level(), Some("debug"));
    }

    #[tokio::test]
    async fn bootstrap_reads_data_plane_log_file() {
        let (config, _) = ConfigurationLoader::for_tests(
            Some(json!({ "data_plane": { "log_file": "/tmp/adp.log" } })),
            None,
            false,
        )
        .await;
        let bc = BootstrapConfiguration::from_configuration(&config).expect("parse bootstrap config");
        assert_eq!(bc.data_plane_log_file.as_deref(), Some("/tmp/adp.log"));
    }

    #[tokio::test]
    async fn bootstrap_permissive_bool_string_true() {
        let (config, _) = ConfigurationLoader::for_tests(
            Some(json!({ "log_format_json": "true", "disable_file_logging": "1" })),
            None,
            false,
        )
        .await;
        let bc = BootstrapConfiguration::from_configuration(&config).expect("parse bootstrap config");
        assert_eq!(bc.log_format_json, Some(true));
        assert!(bc.disable_file_logging);
    }

    #[tokio::test]
    async fn bootstrap_env_overrides_yaml() {
        let env_vars = [("LOG_LEVEL".to_string(), "warn".to_string())];
        let (config, _) =
            ConfigurationLoader::for_tests(Some(json!({ "log_level": "debug" })), Some(&env_vars), false).await;
        let bc = BootstrapConfiguration::from_configuration(&config).expect("parse bootstrap config");
        // env var (TEST_LOG_LEVEL=warn, simulating DD_LOG_LEVEL) overrides YAML log_level=debug
        assert_eq!(bc.log_level.as_deref(), Some("warn"));
    }

    // SalukiPrivateConfiguration tests

    #[tokio::test]
    async fn saluki_private_loads_from_saluki_yaml() {
        use std::io::Write as _;

        use tempfile::NamedTempFile;

        let mut f = NamedTempFile::with_suffix(".yaml").expect("create tempfile");
        writeln!(f, "dogstatsd_port: 9999").expect("write tempfile");

        let priv_config = SalukiPrivateConfiguration::load(Some(f.path())).expect("load saluki private config");
        let port: Option<u64> = priv_config.inner().try_get_typed("dogstatsd_port").unwrap();
        assert_eq!(port, Some(9999));
    }

    #[tokio::test]
    async fn saluki_private_excludes_dd_sources() {
        // SalukiPrivateConfiguration::load() only calls from_environment("SALUKI"),
        // never from_environment("DD_") or any Datadog-schema loader. This test
        // verifies that a config built with no SALUKI_* vars and no saluki.yaml is empty.
        let priv_config = SalukiPrivateConfiguration::load(None).expect("load saluki private config");
        // dogstatsd_port would be absent since no SALUKI_DOGSTATSD_PORT or saluki.yaml set it.
        let port: Option<u64> = priv_config.inner().try_get_typed("dogstatsd_port").unwrap();
        assert!(port.is_none());
    }
}
