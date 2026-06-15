use std::{path::PathBuf, time::{Duration, Instant}};

use agent_data_plane_config::SalukiConfiguration;
use agent_data_plane_config_system::{BootstrapInputs, ConfigurationSystem};
use argh::FromArgs;
use resource_accounting::{ComponentBounds, ComponentRegistry};
use saluki_app::{
    accounting::{initialize_memory_bounds, MemoryBoundsConfiguration},
    bootstrap::BootstrapGuard,
    metrics::emit_startup_metrics,
};
use saluki_core::{
    health::HealthRegistry,
    runtime::{RestartMode, RestartStrategy, Supervisor},
    topology::TopologyBlueprint,
};
use saluki_error::{generic_error, ErrorContext as _, GenericError};
use tracing::{info, warn};

use crate::internal::{DogStatsDControlSurface, TopologyControlSurfaces};
use crate::internal::env::ADPEnvironmentProvider;

/// Runs the data plane.
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "run")]
pub struct RunCommand {
    /// path to the PID file
    #[argh(option, short = 'p', long = "pidfile")]
    pub pid_file: Option<PathBuf>,
}

/// Entrypoint for the `run` command.
///
/// TODO: `main.rs` still builds and passes a `GenericConfiguration`. The new shape should pass
/// `BootstrapInputs` instead, then let `agent-data-plane-config-system` consume all raw local
/// sources before `run.rs` sees anything.
pub async fn handle_run_command(
    started: Instant, bootstrap_inputs: BootstrapInputs, bootstrap_guard: &mut BootstrapGuard,
    bootstrap_supervisor: Supervisor,
) -> Result<(), GenericError> {
    let app_details = saluki_metadata::get_app_details();
    info!(
        version = app_details.version().raw(),
        git_hash = app_details.git_hash(),
        target_arch = app_details.target_arch(),
        build_time = app_details.build_time(),
        process_id = std::process::id(),
        "Agent Data Plane starting..."
    );

    // TODO: This is the intended startup seam. The config system should:
    // - load local bootstrap sources;
    // - parse `BootstrapConfiguration`;
    // - choose `RuntimeConfigAuthority`;
    // - connect/register Datadog Agent only for stream authority;
    // - wait for the initial authoritative snapshot when needed;
    // - load Saluki-private supplemental config;
    // - translate the selected source language into `SalukiConfiguration`;
    // - retain any provider attachments needed for future config updates.
    let config_system = ConfigurationSystem { inputs: bootstrap_inputs };
    let started_config = config_system
        .start()
        .await
        .error_context("Failed to start ADP configuration system.")?;

    let _bootstrap_config = started_config.bootstrap();
    let saluki_config = started_config.saluki();
    let attachments = started_config.attachments();

    // TODO: Runtime logging reload should move behind the config system. Bootstrap logging is still
    // initialized before `run.rs`; runtime logging belongs to `SalukiConfiguration`.
    bootstrap_guard
        .logging_mut()
        .reload(saluki_config.logging.clone())
        .await
        .error_context("Failed to reload runtime logging configuration.")?;

    if !saluki_config.data_plane.enabled() {
        info!("Agent Data Plane is not enabled. Exiting.");
        return Ok(());
    }

    // TODO: Overlay/classifier validation should happen inside the Datadog translator before it
    // returns `SalukiConfiguration`. `run.rs` should never flatten or inspect source-language keys.

    let component_registry = ComponentRegistry::default();
    let health_registry = HealthRegistry::new();

    // TODO: Environment provider construction still depends on `GenericConfiguration` today.
    // The replacement should consume `SalukiConfiguration` plus provider attachments such as the
    // Datadog Agent connection/session handle.
    let (env_provider, maybe_env_supervisor) = ADPEnvironmentProvider::from_saluki_configuration(
        saluki_config,
        attachments,
        &component_registry,
        &health_registry,
    )
    .await?;

    let (mut blueprint, control_surfaces) =
        create_topology(saluki_config, &env_provider, &component_registry).await?;

    // TODO: `create_internal_supervisor` should stop accepting `GenericConfiguration`,
    // `DataPlaneConfiguration`, and bin-local `RemoteAgentBootstrap`. It should receive typed
    // Saluki runtime config plus `StartedAttachments`.
    let mut internal_supervisor = crate::internal::create_internal_supervisor_from_saluki(
        saluki_config,
        attachments,
        &component_registry,
        health_registry.clone(),
        control_surfaces,
        bootstrap_guard.logging().controller(),
    )
    .await
    .error_context("Failed to create internal supervisor.")?;

    // TODO: Memory bounds should be part of `SalukiConfiguration` or derived from it here.
    let bounds_config = MemoryBoundsConfiguration::try_from_saluki_config(saluki_config)?;
    let memory_limiter = initialize_memory_bounds(bounds_config, component_registry.root())?;

    if let Ok(val) = std::env::var("DD_ADP_WRITE_SIZING_GUIDE") {
        if val != "false" {
            if let Err(error) = write_sizing_guide(component_registry.as_bounds()) {
                warn!("Failed to write sizing guide: {}", error);
            } else {
                return Ok(());
            }
        }
    }

    blueprint
        .with_health_registry(health_registry.clone())
        .with_memory_limiter(memory_limiter)
        .with_environment_readiness(env_provider.wait_for_ready());

    let topology_ready = blueprint.topology_ready();

    let root_restart_strategy = RestartStrategy::new(RestartMode::OneForOne, 0, Duration::from_secs(5));
    let mut root_supervisor = Supervisor::new("adp-root")?.with_restart_strategy(root_restart_strategy);

    root_supervisor.add_worker(bootstrap_supervisor);
    if let Some(env_supervisor) = maybe_env_supervisor {
        internal_supervisor.add_worker(env_supervisor);
    }
    root_supervisor.add_worker(internal_supervisor);
    root_supervisor.add_worker(blueprint);

    tokio::spawn(async move {
        if topology_ready.wait().await {
            info!(
                topology_ready_ms = started.elapsed().as_millis(),
                "Topology healthy. Waiting for interrupt..."
            );

            emit_startup_metrics();
        }
    });

    info!("Agent Data Plane running.");
    match root_supervisor.run_with_shutdown(wait_for_sigint()).await {
        Ok(()) => {
            info!("Agent Data Plane shut down successfully.");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

async fn wait_for_sigint() {
    let _ = tokio::signal::ctrl_c().await;

    info!("Received SIGINT, shutting down...");
}

async fn create_topology(
    saluki_config: &SalukiConfiguration, env_provider: &ADPEnvironmentProvider,
    component_registry: &ComponentRegistry,
) -> Result<(TopologyBlueprint, TopologyControlSurfaces), GenericError> {
    let mut blueprint = TopologyBlueprint::new("primary", component_registry);
    let mut control_surfaces = TopologyControlSurfaces::default();

    // TODO: These decisions should come from `SalukiConfiguration`, not the old
    // `DataPlaneConfiguration` accessors.
    if !saluki_config.data_plane.data_pipelines_enabled() {
        return Err(generic_error!("No data pipelines are enabled. Exiting."));
    }

    if saluki_config.data_plane.requires_datadog_forwarder() {
        // TODO: Replace every `from_configuration` constructor with constructors that take native
        // config slices from `SalukiConfiguration`.
        let dd_forwarder_config = saluki_config.forwarder.datadog.clone();
        blueprint.add_forwarder("dd_out", dd_forwarder_config)?;
    }

    if saluki_config.data_plane.metrics_pipeline_required() {
        add_baseline_metrics_pipeline_to_blueprint(&mut blueprint, saluki_config, env_provider).await?;
    }

    if saluki_config.data_plane.logs_pipeline_required() {
        add_baseline_logs_pipeline_to_blueprint(&mut blueprint, saluki_config).await?;
    }

    if saluki_config.data_plane.events_pipeline_required() {
        add_baseline_events_pipeline_to_blueprint(&mut blueprint, saluki_config).await?;
    }

    if saluki_config.data_plane.service_checks_pipeline_required() {
        add_baseline_service_checks_pipeline_to_blueprint(&mut blueprint, saluki_config).await?;
    }

    if saluki_config.data_plane.traces_pipeline_required() {
        add_baseline_traces_pipeline_to_blueprint(&mut blueprint, saluki_config, env_provider).await?;
    }

    if saluki_config.data_plane.checks.enabled() {
        add_checks_pipeline_to_blueprint(&mut blueprint, saluki_config).await?;
    }

    if saluki_config.data_plane.dogstatsd.enabled() {
        let dsd_control_surface = add_dsd_pipeline_to_blueprint(&mut blueprint, saluki_config, env_provider).await?;
        control_surfaces.attach_dogstatsd(dsd_control_surface);
    }

    if saluki_config.data_plane.otlp.enabled() {
        add_otlp_pipeline_to_blueprint(&mut blueprint, saluki_config, env_provider)?;
    }

    Ok((blueprint, control_surfaces))
}

async fn add_checks_pipeline_to_blueprint(
    blueprint: &mut TopologyBlueprint, saluki_config: &SalukiConfiguration,
) -> Result<(), GenericError> {
    // TODO: The checks IPC config should be a native field in `SalukiConfiguration`.
    blueprint
        .add_source("checks_ipc_in", saluki_config.checks.ipc.clone())?
        .connect_components("checks_ipc_in.metrics", "metrics_enrich")?
        .connect_components("checks_ipc_in.logs", "dd_logs_encode")?
        .connect_components("checks_ipc_in.events", "dd_events_encode")?
        .connect_components("checks_ipc_in.service_checks", "dd_service_checks_encode")?;

    Ok(())
}

async fn add_baseline_metrics_pipeline_to_blueprint(
    blueprint: &mut TopologyBlueprint, saluki_config: &SalukiConfiguration, env_provider: &ADPEnvironmentProvider,
) -> Result<(), GenericError> {
    // TODO: Host tags should use the shared Datadog Agent attachment, not create its own IPC client
    // from raw config.
    let metrics_enrich_config = saluki_config.metrics.enrichment.clone().with_environment_provider(env_provider.clone());

    blueprint
        .add_transform("metrics_enrich", metrics_enrich_config)?
        .add_encoder("dd_metrics_encode", saluki_config.metrics.datadog_encoder.clone())?
        .connect_components_in_order(["metrics_enrich", "dd_metrics_encode", "dd_out"])?;

    add_mrf_metrics_pipeline_to_blueprint(blueprint, saluki_config)?;

    Ok(())
}

fn add_mrf_metrics_pipeline_to_blueprint(
    blueprint: &mut TopologyBlueprint, saluki_config: &SalukiConfiguration,
) -> Result<(), GenericError> {
    // TODO: MRF endpoint/API-key refresh must be modeled as a typed capability, not a stored
    // `GenericConfiguration` clone.
    if let Some(mrf) = &saluki_config.metrics.multi_region_failover {
        blueprint
            .add_transform("mrf_metrics_gateway", mrf.gateway.clone())?
            .add_encoder("mrf_metrics_encode", mrf.encoder.clone())?
            .add_forwarder("mrf_dd_out", mrf.forwarder.clone())?
            .connect_components_in_order([
                "metrics_enrich",
                "mrf_metrics_gateway",
                "mrf_metrics_encode",
                "mrf_dd_out",
            ])?;
    }

    Ok(())
}

async fn add_baseline_logs_pipeline_to_blueprint(
    blueprint: &mut TopologyBlueprint, saluki_config: &SalukiConfiguration,
) -> Result<(), GenericError> {
    blueprint
        .add_encoder("dd_logs_encode", saluki_config.logs.datadog_encoder.clone())?
        .connect_components("dd_logs_encode", "dd_out")?;

    Ok(())
}

async fn add_baseline_events_pipeline_to_blueprint(
    blueprint: &mut TopologyBlueprint, saluki_config: &SalukiConfiguration,
) -> Result<(), GenericError> {
    blueprint
        .add_encoder("dd_events_encode", saluki_config.events.datadog_encoder.clone())?
        .connect_components("dd_events_encode", "dd_out")?;

    Ok(())
}

async fn add_baseline_service_checks_pipeline_to_blueprint(
    blueprint: &mut TopologyBlueprint, saluki_config: &SalukiConfiguration,
) -> Result<(), GenericError> {
    blueprint
        .add_encoder("dd_service_checks_encode", saluki_config.service_checks.datadog_encoder.clone())?
        .connect_components("dd_service_checks_encode", "dd_out")?;

    Ok(())
}

async fn add_baseline_traces_pipeline_to_blueprint(
    blueprint: &mut TopologyBlueprint, saluki_config: &SalukiConfiguration, env_provider: &ADPEnvironmentProvider,
) -> Result<(), GenericError> {
    // TODO: Trace/APM config should be fully native, including sampler/obfuscation/private knobs.
    let traces = saluki_config.traces.clone().with_environment_provider(env_provider.clone());

    blueprint
        .add_transform("traces_enrich", traces.enrichment)?
        .add_transform("dd_apm_stats", traces.apm_stats_transform)?
        .add_encoder("dd_stats_encode", traces.apm_stats_encoder)?
        .add_encoder("dd_traces_encode", traces.datadog_encoder)?
        .connect_components("traces_enrich", ["dd_apm_stats", "dd_traces_encode"])?
        .connect_components("dd_apm_stats", "dd_stats_encode")?
        .connect_components(["dd_traces_encode", "dd_stats_encode"], "dd_out")?;

    Ok(())
}

async fn add_dsd_pipeline_to_blueprint(
    blueprint: &mut TopologyBlueprint, saluki_config: &SalukiConfiguration, env_provider: &ADPEnvironmentProvider,
) -> Result<DogStatsDControlSurface, GenericError> {
    // TODO: DogStatsD currently hides much of the Datadog adapter layer in component config
    // constructors. Those adapters should move into the translator.
    let dsd = saluki_config.dogstatsd.clone().with_workload_provider(env_provider.workload().clone());
    let stats_api_handler = dsd.source.stats_api_handler();
    let capture_api_handler = dsd.source.capture_api_handler();
    let replay_api_handler = dsd.source.replay_api_handler();

    blueprint
        .add_source("dsd_in", dsd.source)?
        .add_transform("dsd_prefix_filter", dsd.prefix_filter)?
        .add_transform("dsd_enrich", dsd.enrichment)?
        .add_transform("dsd_tag_filterlist", dsd.tag_filterlist)?
        .add_transform("dsd_agg", dsd.aggregate)?
        .add_transform("dsd_post_agg_filter", dsd.post_aggregate_filter)?
        .add_transform("events_enrich", dsd.events_enrichment)?
        .add_transform("service_checks_enrich", dsd.service_checks_enrichment)?
        .add_destination("dsd_stats_out", dsd.stats_destination)?
        .connect_components_in_order([
            "dsd_in.metrics",
            "dsd_enrich",
            "dsd_prefix_filter",
            "dsd_tag_filterlist",
            "dsd_agg",
            "dsd_post_agg_filter",
            "metrics_enrich",
        ])?
        .connect_components_in_order(["dsd_in.events", "events_enrich", "dd_events_encode"])?
        .connect_components_in_order([
            "dsd_in.service_checks",
            "service_checks_enrich",
            "dd_service_checks_encode",
        ])?
        .connect_components("dsd_in.metrics", "dsd_stats_out")?;

    if let Some(debug_log) = dsd.debug_log {
        blueprint
            .add_destination("dsd_debug_log_out", debug_log)?
            .connect_components("dsd_in.metrics", "dsd_debug_log_out")?;
    }

    Ok(DogStatsDControlSurface {
        stats_api_handler,
        capture_api_handler,
        replay_api_handler,
    })
}

fn add_otlp_pipeline_to_blueprint(
    blueprint: &mut TopologyBlueprint, saluki_config: &SalukiConfiguration, env_provider: &ADPEnvironmentProvider,
) -> Result<(), GenericError> {
    let otlp = saluki_config.otlp.clone().with_workload_provider(env_provider.workload().clone());

    if let Some(proxy) = otlp.proxy {
        info!(
            proxy_grpc_endpoint = %proxy.core_agent_otlp_grpc_endpoint,
            proxy_metrics = proxy.proxy_metrics,
            proxy_logs = proxy.proxy_logs,
            proxy_traces = proxy.proxy_traces,
            "OTLP proxy mode enabled. Select OTLP payloads will be proxied to the Core Agent."
        );

        blueprint
            .add_relay("otlp_relay_in", proxy.relay)?
            .add_forwarder("local_agent_otlp_out", proxy.forwarder)?
            .connect_components(["otlp_relay_in.metrics", "otlp_relay_in.logs"], "local_agent_otlp_out")?;

        if proxy.proxy_traces {
            blueprint.connect_components("otlp_relay_in.traces", "local_agent_otlp_out")?;
        } else {
            blueprint
                .add_decoder("otlp_traces_decode", proxy.traces_decoder)?
                .connect_components_in_order(["otlp_relay_in.traces", "otlp_traces_decode", "traces_enrich"])?;
        }
    } else {
        info!("OTLP proxy mode disabled. OTLP signals will be handled natively.");

        blueprint
            .add_source("otlp_in", otlp.source)?
            .connect_components("otlp_in.metrics", "metrics_enrich")?
            .connect_components("otlp_in.logs", "dd_logs_encode")?
            .connect_components("otlp_in.traces", "traces_enrich")?;
    }

    Ok(())
}

fn write_sizing_guide(bounds: ComponentBounds) -> Result<(), GenericError> {
    use std::{
        fs::File,
        io::{BufWriter, Write},
    };

    let template = include_str!("../sizing_guide_template.html");
    let mut output = BufWriter::new(File::create("sizing_guide.html")?);
    for line in template.lines() {
        if line.trim() == "<!-- INSERT GENERATED CONTENT -->" {
            serde_json::to_writer_pretty(&mut output, &bounds.to_exprs())?;
        } else {
            output.write_all(line.as_bytes())?;
        }
        output.write_all(b"\n")?;
    }
    info!("Wrote sizing guide to sizing_guide.html");
    output.flush()?;

    Ok(())
}
