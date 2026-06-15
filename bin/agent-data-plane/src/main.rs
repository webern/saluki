//! Main benchmarking binary.
//!
//! This binary emulates the standalone DogStatsD binary, listening for DogStatsD over UDS, aggregating metrics over a
//! 10 second window, and shipping those metrics to the Datadog Platform.

#![deny(warnings)]
#![deny(missing_docs)]
use std::time::Instant;

// Pull in the Antithesis coverage-instrumentation runtime shim only when
// building for antithesis. Load-baring: equired to avoid the shim being dropped
// as unused.
#[cfg(feature = "antithesis")]
use antithesis_instrumentation as _;
use datadog_agent_commons::platform::PlatformSettings;
use metrics::Level;
use saluki_app::bootstrap::{AppBootstrapper, Bootstrap, BootstrapGuard};
use saluki_components::config::{DatadogRemapper, KEY_ALIASES};
use saluki_config::{ConfigurationLoader, GenericConfiguration};
use saluki_core::runtime::Supervisor;
use saluki_error::{generic_error, ErrorContext as _, GenericError};
use tracing::{error, info, warn};

mod cli;
use self::cli::*;
use crate::config::BootstrapConfiguration;
use crate::internal::logging::LoggingConfigurationTranslator;

mod components;
mod config;
mod internal;

pub(crate) mod state;

#[cfg(all(target_os = "linux", not(system_allocator)))]
#[global_allocator]
static ALLOC: resource_accounting::TrackingAllocator<tikv_jemallocator::Jemalloc> =
    resource_accounting::TrackingAllocator::new(tikv_jemallocator::Jemalloc);

#[cfg(any(not(target_os = "linux"), system_allocator))]
#[global_allocator]
static ALLOC: resource_accounting::TrackingAllocator<std::alloc::System> =
    resource_accounting::TrackingAllocator::new(std::alloc::System);

#[tokio::main]
async fn main() -> Result<(), GenericError> {
    let started = Instant::now();

    // Initialize the Antithesis SDK as early as possible so assertions and lifecycle hooks register
    // their catalog before any are evaluated. No-op outside Antithesis and absent in production builds.
    #[cfg(feature = "antithesis")]
    antithesis_sdk::antithesis_init();

    let cli: Cli = argh::from_env();

    // Print version and exit early without requiring config.
    if let Action::Version(v) = &cli.action {
        handle_version_command(v.json).await;
        return Ok(());
    }

    // Load our "bootstrap" configuration -- static configuration on disk or from environment variables -- so we can
    // initialize basic subsystems before executing the given subcommand.
    //
    // Two views of the same source set are built here:
    //   - `bootstrap_config`: a typed `BootstrapConfiguration` covering exactly the keys needed
    //     before the Agent config stream is available. Used for the bootstrap-phase operations
    //     (logging init, metrics level). This is the allowlisted pre-authority surface.
    //   - `generic_config`: an untyped `GenericConfiguration` over the same sources. Forwarded to
    //     `run_inner` for the runtime configuration path, which still reads many keys generically.
    //     Will be eliminated when `GenericConfiguration` is confined to the translation system in
    //     PR 11.
    let bootstrap_config_path = cli.config_file.unwrap_or_else(PlatformSettings::get_config_file_path);
    let bootstrap_loader = ConfigurationLoader::default()
        .with_key_aliases(KEY_ALIASES)
        .from_yaml(&bootstrap_config_path)
        .error_context("Failed to load Datadog Agent configuration file during bootstrap.")?
        .add_providers([DatadogRemapper::new()])
        .from_environment(PlatformSettings::get_env_var_prefix())
        .error_context("Environment variable prefix should not be empty.")?;

    let generic_config = bootstrap_loader.bootstrap_generic();
    let bootstrap_config = BootstrapConfiguration::from_configuration(&generic_config)
        .error_context("Failed to build typed bootstrap configuration.")?;

    // Translate the typed bootstrap configuration into ADP's logging configuration, applying ADP-specific
    // rules (per-subagent log file key, never sharing a file with the Core Agent).
    let bootstrap_logging_config = LoggingConfigurationTranslator::from_bootstrap(&bootstrap_config)
        .error_context("Failed to translate logging configuration during bootstrap phase.")?;

    let metrics_default_level = parse_metrics_level(&bootstrap_config)?;

    // Proceed with bootstrapping.
    //
    // This initializes logging, metrics, allocator telemetry, TLS, and more. We get handled a guard that we need to
    // hold until the application is about to exit, which ensures things like flushing any buffered logs, and so on.
    let bootstrapper = AppBootstrapper::new()
        .error_context("Failed to initialize bootstrapper.")?
        .with_metrics_prefix("adp")
        .with_metrics_default_level(metrics_default_level)
        .with_logging_configuration(bootstrap_logging_config);
    let Bootstrap {
        supervisor: bootstrap_supervisor,
        guard: mut bootstrap_guard,
    } = bootstrapper
        .bootstrap()
        .await
        .error_context("Failed to complete bootstrap phase.")?;

    // Bootstrap-integration probe: proves the Antithesis SDK is linked, cataloging works, and the
    // instrumentation path is wired.
    #[cfg(feature = "antithesis")]
    antithesis_sdk::assert_reachable!("agent-data-plane completed bootstrap", &serde_json::json!({}));

    // Run the given subcommand. The bootstrap supervisor is forwarded by value; only the long-lived `run`
    // subcommand actually drives it (it is added as a child of the internal supervisor inside
    // `handle_run_command`). All other subcommands drop it on entry.
    let maybe_exit_code = run_inner(
        cli.action,
        started,
        bootstrap_config,
        generic_config,
        &mut bootstrap_guard,
        bootstrap_supervisor,
    )
    .await?;

    // Drop the bootstrap guard to ensure logs are flushed, etc.
    drop(bootstrap_guard);

    // Exit with the specific exit code, if one was provided.
    if let Some(exit_code) = maybe_exit_code {
        std::process::exit(exit_code);
    }

    Ok(())
}

fn parse_metrics_level(config: &BootstrapConfiguration) -> Result<Level, GenericError> {
    match config.metrics_level() {
        Some(value) => Level::try_from(value).map_err(|e| generic_error!("Failed to parse `metrics_level`: {}", e)),
        None => Ok(Level::INFO),
    }
}

async fn run_inner(
    action: Action, started: Instant, bootstrap_config: BootstrapConfiguration, generic_config: GenericConfiguration,
    bootstrap_guard: &mut BootstrapGuard, bootstrap_supervisor: Supervisor,
) -> Result<Option<i32>, GenericError> {
    match action {
        Action::Run(cmd) => {
            // Populate our PID file, if configured.
            if let Some(pid_file) = &cmd.pid_file {
                let pid = std::process::id();
                if let Err(e) = std::fs::write(pid_file, pid.to_string()) {
                    error!(error = %e, path = %pid_file.display(), "Failed to update PID file. Exiting.");
                    return Ok(Some(1));
                }
            }

            let exit_code = match handle_run_command(
                started,
                bootstrap_config,
                generic_config,
                bootstrap_guard,
                bootstrap_supervisor,
            )
            .await
            {
                Ok(()) => {
                    info!("Agent Data Plane stopped.");
                    None
                }
                Err(e) => {
                    error!("{:?}", e);
                    Some(1)
                }
            };

            // Remove the PID file, if configured.
            if let Some(pid_file) = &cmd.pid_file {
                if let Err(e) = std::fs::remove_file(pid_file) {
                    warn!(error = %e, path = %pid_file.display(), "Failed to delete PID file while exiting.");
                }
            }

            if let Some(exit_code) = exit_code {
                return Ok(Some(exit_code));
            }
        }
        Action::Debug(cmd) => handle_debug_command(&generic_config, cmd).await,
        Action::Config(_) => handle_config_command(&generic_config).await,
        Action::Dogstatsd(cmd) => handle_dogstatsd_command(&generic_config, cmd).await,
        Action::Version(v) => handle_version_command(v.json).await,
    }

    Ok(None)
}
