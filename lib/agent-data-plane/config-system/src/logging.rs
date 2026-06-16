//! Datadog-source logging translation and dynamic log-level update bridge.

use async_trait::async_trait;
use bytesize::ByteSize;
use datadog_agent_commons::platform::PlatformSettings;
use saluki_app::logging::{LogLevel, LoggingConfiguration, LoggingOverrideController};
use saluki_common::{deser::PermissiveBool, sync::shutdown::ShutdownHandle};
use saluki_config::GenericConfiguration;
use saluki_core::runtime::{InitializationError, Supervisable, SupervisorFuture};
use saluki_error::{ErrorContext as _, GenericError};
use serde::Deserialize;
use serde_with::serde_as;
use tokio::{pin, select};
use tracing::{debug, warn};

const DATA_PLANE_LOG_FILE_KEY: &str = "data_plane.log_file";

const FIRST_PARTY_LOG_TARGETS: &[&str] = &[
    "agent_data_plane",
    "containerd_protos",
    "datadog_protos",
    "datadog_agent_commons",
    "ddsketch",
    "resource_accounting",
    "otlp_protos",
    "ottl",
    "process_memory",
    "prometheus_exposition",
    "saluki_api",
    "saluki_app",
    "saluki_common",
    "saluki_components",
    "saluki_config",
    "saluki_context",
    "saluki_core",
    "saluki_env",
    "saluki_error",
    "saluki_io",
    "saluki_metadata",
    "saluki_metrics",
    "saluki_tls",
    "stringtheory",
];

/// Translates Datadog-shaped source configuration into ADP logging configuration.
pub struct LoggingConfigurationTranslator;

impl LoggingConfigurationTranslator {
    /// Builds a [`LoggingConfiguration`] from a Datadog-shaped source snapshot.
    pub fn translate(config: &GenericConfiguration) -> Result<LoggingConfiguration, GenericError> {
        let mut logging = LoggingConfiguration::simple();

        let maybe_log_level = config
            .try_get_typed::<String>("log_level")
            .error_context("Failed to read `log_level`.")?;
        logging.log_level = parse_optional_log_level_raw(maybe_log_level)?;

        if let Some(format_json) = read_permissive_bool(config, "log_format_json")? {
            logging.log_format_json = format_json;
        }

        if let Some(format_rfc3339) = read_permissive_bool(config, "log_format_rfc3339")? {
            logging.log_format_rfc3339 = format_rfc3339;
        }

        if let Some(to_console) = read_permissive_bool(config, "log_to_console")? {
            logging.log_to_console = to_console;
        }

        if let Some(to_syslog) = read_permissive_bool(config, "log_to_syslog")? {
            logging.log_to_syslog = to_syslog;
        }

        if logging.log_to_syslog {
            if let Some(syslog_rfc) = read_permissive_bool(config, "syslog_rfc")? {
                logging.syslog_rfc = syslog_rfc;
            }

            let configured = config
                .try_get_typed::<String>("syslog_uri")
                .error_context("Failed to read `syslog_uri`.")?;
            logging.syslog_uri = match configured {
                Some(uri) if !uri.is_empty() => uri,
                _ => PlatformSettings::get_default_syslog_uri().to_string(),
            };
        }

        if let Some(max_size) = config
            .try_get_typed::<ByteSize>("log_file_max_size")
            .error_context("Failed to read `log_file_max_size`.")?
        {
            logging.log_file_max_size = max_size;
        }

        if let Some(max_rolls) = config
            .try_get_typed::<usize>("log_file_max_rolls")
            .error_context("Failed to read `log_file_max_rolls`.")?
        {
            logging.log_file_max_rolls = max_rolls;
        }

        let disable_file_logging = read_permissive_bool(config, "disable_file_logging")?.unwrap_or(false);
        logging.log_file = if disable_file_logging {
            String::new()
        } else {
            let configured = config
                .try_get_typed::<String>(DATA_PLANE_LOG_FILE_KEY)
                .with_error_context(|| format!("Failed to read `{}`.", DATA_PLANE_LOG_FILE_KEY))?;
            match configured {
                Some(path) if !path.is_empty() => path,
                _ => PlatformSettings::get_default_log_file_path()
                    .to_string_lossy()
                    .into_owned(),
            }
        };

        Ok(logging)
    }
}

fn read_permissive_bool(config: &GenericConfiguration, key: &str) -> Result<Option<bool>, GenericError> {
    Ok(config
        .try_get_typed::<PermissiveBoolValue>(key)
        .with_error_context(|| format!("Failed to read `{}`.", key))?
        .map(|v| v.0))
}

#[serde_as]
#[derive(Deserialize)]
struct PermissiveBoolValue(#[serde_as(as = "PermissiveBool")] bool);

fn parse_optional_log_level_raw(maybe_log_level: Option<String>) -> Result<LogLevel, GenericError> {
    match maybe_log_level {
        Some(log_level) => parse_adp_log_level(&log_level),
        None => first_party_log_level_filter("info"),
    }
}

fn parse_adp_log_level(value: &str) -> Result<LogLevel, GenericError> {
    let trimmed = value.trim();
    if let Some(level) = plain_log_level(trimmed) {
        first_party_log_level_filter(level)
    } else {
        LogLevel::try_from(value.to_string()).error_context("Failed to parse log filter directives.")
    }
}

fn plain_log_level(value: &str) -> Option<&'static str> {
    match value.to_ascii_lowercase().as_str() {
        "trace" => Some("trace"),
        "debug" => Some("debug"),
        "info" => Some("info"),
        "warn" => Some("warn"),
        "error" => Some("error"),
        "off" => Some("off"),
        _ => None,
    }
}

fn first_party_log_level_filter(level: &str) -> Result<LogLevel, GenericError> {
    let filter = FIRST_PARTY_LOG_TARGETS
        .iter()
        .map(|target| format!("{target}={level}"))
        .collect::<Vec<_>>()
        .join(",");

    LogLevel::try_from(filter).error_context("Failed to parse first-party log filter directives.")
}

/// Worker that watches Datadog-source `log_level` updates and applies them to ADP logging.
pub struct DynamicLogLevelWorker {
    config: GenericConfiguration,
    controller: LoggingOverrideController,
}

impl DynamicLogLevelWorker {
    /// Creates a worker backed by the given source configuration snapshot/update stream.
    pub fn new(config: &GenericConfiguration, controller: LoggingOverrideController) -> Self {
        Self {
            config: config.clone(),
            controller,
        }
    }
}

#[async_trait]
impl Supervisable for DynamicLogLevelWorker {
    fn name(&self) -> &str {
        "dynamic-log-level"
    }

    async fn initialize(&self, process_shutdown: ShutdownHandle) -> Result<SupervisorFuture, InitializationError> {
        let mut watcher = self.config.watch_for_updates("log_level");
        let controller = self.controller.clone();

        Ok(Box::pin(async move {
            pin!(process_shutdown);

            debug!("Dynamic log level worker started.");

            loop {
                select! {
                    _ = &mut process_shutdown => break,
                    (_, new_log_level) = watcher.changed::<String>() => {
                        match parse_optional_log_level_raw(new_log_level) {
                            Ok(log_level) => {
                                if let Err(e) = controller.update_base(log_level.as_env_filter()).await {
                                    warn!(error = %e, %log_level, "Failed to apply updated log level.");
                                }
                            }
                            Err(e) => warn!(error = %e, "Failed to parse updated log level."),
                        }
                    }
                }
            }

            debug!("Dynamic log level worker stopped.");

            Ok(())
        }))
    }
}
