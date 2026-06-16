use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use agent_data_plane_config::SalukiConfiguration;
use agent_data_plane_config_system::{BootstrapInputs, ConfigurationSystem, StartedAttachments};
use argh::FromArgs;
use datadog_agent_commons::platform::PlatformSettings;
use resource_accounting::{ComponentBounds, ComponentRegistry};
use saluki_app::{
    accounting::{initialize_memory_bounds, MemoryBoundsConfiguration},
    bootstrap::BootstrapGuard,
    metrics::emit_startup_metrics,
};
use saluki_components::{
    config::MrfConfiguration,
    decoders::otlp::OtlpDecoderConfiguration,
    destinations::{DogStatsDDebugLogConfiguration, DogStatsDStatisticsConfiguration},
    encoders::{
        BufferedIncrementalConfiguration, DatadogApmStatsEncoderConfiguration, DatadogEventsConfiguration,
        DatadogLogsConfiguration, DatadogMetricsConfiguration, DatadogServiceChecksConfiguration,
        DatadogTraceConfiguration,
    },
    forwarders::{DatadogForwarderConfiguration, OtlpForwarderConfiguration},
    relays::otlp::OtlpRelayConfiguration,
    sources::{ChecksIPCConfiguration, DogStatsDConfiguration, OtlpConfiguration},
    transforms::{
        AggregateConfiguration, ApmStatsTransformConfiguration, ChainedConfiguration, DogStatsDMapperConfiguration,
        HostEnrichmentConfiguration, MrfMetricsGatewayConfiguration, TraceObfuscationConfiguration,
        TraceSamplerConfiguration,
    },
};
use saluki_config::GenericConfiguration;
use saluki_core::health::HealthRegistry;
use saluki_core::runtime::{RestartMode, RestartStrategy, Supervisor};
use saluki_core::topology::TopologyBlueprint;
use saluki_env::EnvironmentProvider as _;
use saluki_error::{generic_error, ErrorContext as _, GenericError};
use tracing::{info, warn};

use crate::internal::env::ADPEnvironmentProvider;
use crate::{
    components::{
        apm_onboarding::ApmOnboardingConfiguration,
        dogstatsd_post_aggregate_filter::DogStatsDPostAggregateFilterConfiguration,
        dogstatsd_prefix_filter::DogStatsDPrefixFilterConfiguration, host_tags::HostTagsConfiguration,
        ottl_filter_processor::OttlFilterConfiguration, ottl_transform_processor::OttlTransformConfiguration,
        tag_filterlist::TagFilterlistConfiguration,
    },
    internal::{
        create_internal_supervisor, logging::LoggingConfigurationTranslator, remote_agent::RemoteAgentBootstrap,
        DogStatsDControlSurface, TopologyControlSurfaces,
    },
};

/// Runs the data plane.
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "run")]
pub struct RunCommand {
    /// path to the PID file
    #[argh(option, short = 'p', long = "pidfile")]
    pub pid_file: Option<PathBuf>,
}

/// Entrypoint for the `run` commands.
pub async fn handle_run_command(
    started: Instant, bootstrap_config: GenericConfiguration, bootstrap_inputs: BootstrapInputs,
    bootstrap_guard: &mut BootstrapGuard, bootstrap_supervisor: Supervisor,
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

    let started_config = ConfigurationSystem {
        inputs: bootstrap_inputs,
    }
    .start_from_local_datadog_sources(bootstrap_config)
    .await
    .error_context("Failed to start configuration system.")?;
    let config = started_config.compat_datadog_source();
    let saluki_config = started_config.saluki();
    let dp_config = &saluki_config.data_plane;

    match LoggingConfigurationTranslator::translate(config) {
        Ok(logging_config) => {
            if let Err(e) = bootstrap_guard.logging_mut().reload(logging_config).await {
                warn!(
                    error = %e,
                    "Failed to reload logging from resolved configuration; continuing with bootstrap logging settings."
                );
            }
        }
        Err(e) => warn!(
            error = %e,
            "Failed to translate logging configuration from resolved configuration; continuing with bootstrap logging settings."
        ),
    }

    if !dp_config.standalone_mode() && !dp_config.enabled() {
        info!("Agent Data Plane is not enabled. Exiting.");
        return Ok(());
    }

    // Set up all of the building blocks for building our topologies and launching internal processes.
    let component_registry = ComponentRegistry::default();
    let health_registry = HealthRegistry::new();
    let (env_provider, maybe_env_supervisor) = ADPEnvironmentProvider::from_saluki_configuration(
        saluki_config,
        started_config.attachments(),
        &component_registry,
        &health_registry,
    )
    .await?;

    // Create the blueprint for our primary topology.
    let (mut blueprint, control_surfaces) =
        create_topology(config, saluki_config, &env_provider, &component_registry).await?;

    // Create the internal supervisor which drives our control plane and internal observability.
    let mut internal_supervisor = create_internal_supervisor(
        config,
        dp_config,
        &saluki_config.control_plane,
        &component_registry,
        health_registry.clone(),
        control_surfaces,
        remote_agent_bootstrap_from_attachments(started_config.attachments()).await,
        bootstrap_guard.logging().controller(),
    )
    .await
    .error_context("Failed to create internal supervisor.")?;

    // Run memory bounds validation to ensure that we can launch the topology with our configured memory limit, if any.
    let bounds_config = MemoryBoundsConfiguration::try_from_config(config)?;
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

    // Assemble the full supervision tree and run it.
    //
    // We run our internal supervisor (control plane, environment provider, etc) and our topology supervisor
    // side-by-side, which means everyone has access to the same dataspace. This is crucial for allowing processes
    // in the topology supervisor to (eventually) interact with components in the control plane, and vise versa.
    blueprint
        .with_health_registry(health_registry.clone())
        .with_memory_limiter(memory_limiter)
        .with_environment_readiness(env_provider.wait_for_ready());

    // Acquire a readiness handle before handing the blueprint off to the supervisor. This waits until the topology has
    // registered its components in the health registry and they've all reported ready, rather than racing the supervisor
    // and potentially observing an empty/already-ready registry before the topology's components even exist.
    let topology_ready = blueprint.topology_ready();

    let root_restart_strategy = RestartStrategy::new(RestartMode::OneForOne, 0, Duration::from_secs(5));
    let mut root_supervisor = Supervisor::new("adp-root")?.with_restart_strategy(root_restart_strategy);

    root_supervisor.add_worker(bootstrap_supervisor);
    if let Some(env_supervisor) = maybe_env_supervisor {
        internal_supervisor.add_worker(env_supervisor);
    }
    root_supervisor.add_worker(internal_supervisor);
    root_supervisor.add_worker(blueprint);

    // Once the topology is healthy, log readiness and emit our startup metrics.
    tokio::spawn(async move {
        // If the topology is torn down before it ever becomes ready (for example, shutdown during startup), `wait`
        // returns `false` and we skip reporting readiness.
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

async fn remote_agent_bootstrap_from_attachments(attachments: &StartedAttachments) -> Option<RemoteAgentBootstrap> {
    match attachments {
        StartedAttachments::DatadogAgentConfigStream { connection, .. } => {
            Some(RemoteAgentBootstrap::from_datadog_connection(connection).await)
        }
        StartedAttachments::None => None,
    }
}

async fn create_topology(
    config: &GenericConfiguration, saluki_config: &SalukiConfiguration, env_provider: &ADPEnvironmentProvider,
    component_registry: &ComponentRegistry,
) -> Result<(TopologyBlueprint, TopologyControlSurfaces), GenericError> {
    let dp_config = &saluki_config.data_plane;
    let mut blueprint = TopologyBlueprint::new("primary", component_registry);
    let mut control_surfaces = TopologyControlSurfaces::default();

    // If no data pipelines are enabled, then there's nothing for us to do.
    if !dp_config.data_pipelines_enabled() {
        return Err(generic_error!("No data pipelines are enabled. Exiting."));
    }

    // Create our baseline pipelines if necessary.
    //
    // We check if the "metrics" or "logs" pipeline is required, which represent the basic components necessary to
    // forward metrics and logs to Datadog. This means that if either are enabled, we always create the forwarder, but
    // we additionally create metrics- and logs-specific components connected to that forwarder depending on which of
    // the baseline pipelines are required.
    //
    // Notably, we _don't_ need either of these if all we're doing is running the OTLP pipeline in proxy mode, which
    // is the only reason we're differentiating here.
    if dp_config.metrics_pipeline_required()
        || dp_config.logs_pipeline_required()
        || dp_config.events_pipeline_required()
        || dp_config.service_checks_pipeline_required()
        || dp_config.traces_pipeline_required()
    {
        let dd_forwarder_config = DatadogForwarderConfiguration::from_configuration(config)
            .error_context("Failed to configure Datadog forwarder.")?;
        blueprint.add_forwarder("dd_out", dd_forwarder_config)?;
    }

    if dp_config.metrics_pipeline_required() {
        add_baseline_metrics_pipeline_to_blueprint(&mut blueprint, config, saluki_config, env_provider).await?;
    }

    if dp_config.logs_pipeline_required() {
        add_baseline_logs_pipeline_to_blueprint(&mut blueprint, saluki_config).await?;
    }

    if dp_config.events_pipeline_required() {
        add_baseline_events_pipeline_to_blueprint(&mut blueprint, saluki_config).await?;
    }

    if dp_config.service_checks_pipeline_required() {
        add_baseline_service_checks_pipeline_to_blueprint(&mut blueprint, saluki_config).await?;
    }

    if dp_config.traces_pipeline_required() {
        add_baseline_traces_pipeline_to_blueprint(&mut blueprint, config, saluki_config, env_provider).await?;
    }

    // Now we move on to our actual data pipelines.
    if dp_config.checks().enabled() {
        add_checks_pipeline_to_blueprint(&mut blueprint, saluki_config).await?;
    }

    if dp_config.dogstatsd().enabled() {
        let dsd_control_surface = add_dsd_pipeline_to_blueprint(&mut blueprint, config, env_provider).await?;
        control_surfaces.attach_dogstatsd(dsd_control_surface);
    }

    if dp_config.otlp().enabled() {
        add_otlp_pipeline_to_blueprint(&mut blueprint, saluki_config, env_provider)?;
    }

    Ok((blueprint, control_surfaces))
}

async fn add_checks_pipeline_to_blueprint(
    blueprint: &mut TopologyBlueprint, config: &SalukiConfiguration,
) -> Result<(), GenericError> {
    let checks_config = ChecksIPCConfiguration::from_native(&config.checks_ipc);

    blueprint
        .add_source("checks_ipc_in", checks_config)?
        .connect_components("checks_ipc_in.metrics", "metrics_enrich")?
        .connect_components("checks_ipc_in.logs", "dd_logs_encode")?
        .connect_components("checks_ipc_in.events", "dd_events_encode")?
        .connect_components("checks_ipc_in.service_checks", "dd_service_checks_encode")?;

    Ok(())
}

async fn add_baseline_metrics_pipeline_to_blueprint(
    blueprint: &mut TopologyBlueprint, config: &GenericConfiguration, saluki_config: &SalukiConfiguration,
    env_provider: &ADPEnvironmentProvider,
) -> Result<(), GenericError> {
    // Create the back half of the metrics processing pipeline.
    let host_enrichment_config = HostEnrichmentConfiguration::from_environment_provider(env_provider.clone());
    let mut metrics_enrich_config =
        ChainedConfiguration::default().with_transform_builder("host_enrichment", host_enrichment_config);

    if let Some(client) = env_provider.datadog_agent_client() {
        let host_tags_config =
            HostTagsConfiguration::from_client(client, saluki_config.environment.host_tags_expected_tags_duration());
        metrics_enrich_config = metrics_enrich_config.with_transform_builder("host_tags", host_tags_config);
    }

    let dd_metrics_config = DatadogMetricsConfiguration::from_configuration(config)
        .error_context("Failed to configure Datadog Metrics encoder.")?;

    blueprint
        // Components.
        .add_transform("metrics_enrich", metrics_enrich_config)?
        .add_encoder("dd_metrics_encode", dd_metrics_config)?
        // Metrics, then forwarding.
        .connect_components_in_order(["metrics_enrich", "dd_metrics_encode", "dd_out"])?;

    add_mrf_metrics_pipeline_to_blueprint(blueprint, config)?;

    Ok(())
}

fn add_mrf_metrics_pipeline_to_blueprint(
    blueprint: &mut TopologyBlueprint, config: &GenericConfiguration,
) -> Result<(), GenericError> {
    let mrf_config = MrfConfiguration::from_configuration(config)
        .error_context("Failed to configure Multi-Region Failover metrics pipeline.")?;

    let Some((mrf_dd_url, mrf_api_key)) = mrf_config.metrics_endpoint_override() else {
        if mrf_config.is_enabled() {
            warn!(
                "Multi-Region Failover is enabled, but multi_region_failover.api_key and one of \
                 multi_region_failover.dd_url or multi_region_failover.site are required for metrics forwarding. The \
                 MRF metrics branch will not be wired, and primary forwarding will continue. Restart ADP after \
                 configuring the static MRF endpoint settings."
            );
        }

        return Ok(());
    };

    let mrf_gateway_config = MrfMetricsGatewayConfiguration::new(mrf_config.clone(), config.clone());
    let mrf_metrics_config = DatadogMetricsConfiguration::from_configuration(config)
        .error_context("Failed to configure Multi-Region Failover Datadog Metrics encoder.")?;

    let mrf_forwarder_config = DatadogForwarderConfiguration::from_configuration(config)
        .map(|config| {
            config.with_endpoint_override_and_api_key_refresh_config_path(
                mrf_dd_url,
                mrf_api_key,
                "multi_region_failover.api_key",
            )
        })
        .error_context("Failed to configure Multi-Region Failover Datadog forwarder.")?;

    blueprint
        .add_transform("mrf_metrics_gateway", mrf_gateway_config)?
        .add_encoder("mrf_metrics_encode", mrf_metrics_config)?
        .add_forwarder("mrf_dd_out", mrf_forwarder_config)?
        .connect_components_in_order([
            "metrics_enrich",
            "mrf_metrics_gateway",
            "mrf_metrics_encode",
            "mrf_dd_out",
        ])?;

    Ok(())
}

async fn add_baseline_logs_pipeline_to_blueprint(
    blueprint: &mut TopologyBlueprint, config: &SalukiConfiguration,
) -> Result<(), GenericError> {
    // Create the back half of the logs processing pipeline.
    let dd_logs_config = BufferedIncrementalConfiguration::from_encoder_builder(DatadogLogsConfiguration::from_native(
        &config.datadog_logs_encoder,
    ));

    blueprint
        // Components.
        .add_encoder("dd_logs_encode", dd_logs_config)?
        // Logs.
        .connect_components("dd_logs_encode", "dd_out")?;

    Ok(())
}

async fn add_baseline_events_pipeline_to_blueprint(
    blueprint: &mut TopologyBlueprint, config: &SalukiConfiguration,
) -> Result<(), GenericError> {
    let dd_events_config = BufferedIncrementalConfiguration::from_encoder_builder(
        DatadogEventsConfiguration::from_native(&config.datadog_events_encoder),
    );

    blueprint
        .add_encoder("dd_events_encode", dd_events_config)?
        .connect_components("dd_events_encode", "dd_out")?;

    Ok(())
}

async fn add_baseline_service_checks_pipeline_to_blueprint(
    blueprint: &mut TopologyBlueprint, config: &SalukiConfiguration,
) -> Result<(), GenericError> {
    let dd_service_checks_config = BufferedIncrementalConfiguration::from_encoder_builder(
        DatadogServiceChecksConfiguration::from_native(&config.datadog_service_checks_encoder),
    );

    blueprint
        .add_encoder("dd_service_checks_encode", dd_service_checks_config)?
        .connect_components("dd_service_checks_encode", "dd_out")?;

    Ok(())
}

async fn add_baseline_traces_pipeline_to_blueprint(
    blueprint: &mut TopologyBlueprint, config: &GenericConfiguration, saluki_config: &SalukiConfiguration,
    env_provider: &ADPEnvironmentProvider,
) -> Result<(), GenericError> {
    let dd_traces_config = DatadogTraceConfiguration::from_configuration(config)
        .error_context("Failed to configure Datadog Traces encoder.")?
        .with_environment_provider(env_provider.clone())
        .await?;
    let trace_obfuscation_config = TraceObfuscationConfiguration::from_apm_configuration(config)?;
    let trace_sampler_config = TraceSamplerConfiguration::from_configuration(config)
        .error_context("Failed to configure Trace Sampler transform.")?;
    let ottl_filter_config = OttlFilterConfiguration::from_native(&saluki_config.ottl_filter);
    let ottl_transform_config = OttlTransformConfiguration::from_native(&saluki_config.ottl_transform);
    let dd_traces_enrich_config = ChainedConfiguration::default()
        .with_transform_builder("ottl_filter", ottl_filter_config)
        .with_transform_builder("ottl_transform", ottl_transform_config)
        .with_transform_builder("apm_onboarding", ApmOnboardingConfiguration)
        .with_transform_builder("trace_obfuscation", trace_obfuscation_config)
        .with_transform_builder("trace_sampler", trace_sampler_config);
    let apm_stats_transform_config = ApmStatsTransformConfiguration::from_configuration(config)
        .error_context("Failed to configure APM Stats transform.")?
        .with_environment_provider(env_provider.clone())
        .await?;
    let dd_apm_stats_encoder = DatadogApmStatsEncoderConfiguration::from_configuration(config)
        .error_context("Failed to configure Datadog APM Stats encoder.")?
        .with_environment_provider(env_provider.clone())
        .await?;

    blueprint
        .add_transform("traces_enrich", dd_traces_enrich_config)?
        .add_transform("dd_apm_stats", apm_stats_transform_config)?
        .add_encoder("dd_stats_encode", dd_apm_stats_encoder)?
        .add_encoder("dd_traces_encode", dd_traces_config)?
        .connect_components("traces_enrich", ["dd_apm_stats", "dd_traces_encode"])?
        .connect_components("dd_apm_stats", "dd_stats_encode")?
        .connect_components(["dd_traces_encode", "dd_stats_encode"], "dd_out")?;

    Ok(())
}

async fn add_dsd_pipeline_to_blueprint(
    blueprint: &mut TopologyBlueprint, config: &GenericConfiguration, env_provider: &ADPEnvironmentProvider,
) -> Result<DogStatsDControlSurface, GenericError> {
    // We're creating the "front half" of the DogStatsD pipeline, which deals solely with accepting DogStatsD payloads,
    // and enriching/processing them in DSD-specific ways, relevant to how the Datadog Agent is expected to behave.
    //
    //                                                 ┌─────────────────────┐
    //                              metrics            │      DogStatsD      │
    //               ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ │       (source)      │ ─ ─ ─ ─ ─ ─ ─ ┐
    //               │                 │               └─────────────────────┘               │
    //               │                 │                          │                          │
    //               │                 │                          │ service checks           │ events
    //               │                 ▼                          ▼                          ▼
    //               │      ┌─────────────────────┐    ┌─────────────────────┐    ┌─────────────────────┐
    //               │      │     DSD Enrich      │    │     DSD Service     │    │     DSD Events      │
    //               │      │ (chained transform) │    │    Checks (encoder) │    │      (encoder)      │
    //               │      │┌───────────────────┐│    └─────────────────────┘    └─────────────────────┘
    //               │      ││    DSD Mapper     ││               │                          │
    //               │      │└───────────────────┘│               │                          │
    //               │      └─────────────────────┘               │                          │
    //               │                 │                          │                          │
    //               │                 ▼                          │                          │
    //               │      ┌─────────────────────┐               │                          │
    //               │      │  DSD Prefix/Filter  │               │                          │
    //               │      │     (transform)     │               │                          │
    //               │      └─────────────────────┘               │                          │
    //               │                 │                          │                          │
    //               │                 │        ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐           │
    //               │                 └ ─ ─ ─▶ │        Metrics Pipeline       │           │
    //               │                          │  (aggregate, enrich, encode)  │           │
    //               │                          └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘           │
    //               │                                       │                               │
    //               ▼                                       ▼                               ▼
    //    ┌─────────────────────┐    ┌ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┐
    //    │      DSD Stats      │    │                           Forwarder                             │
    //    │    (destination)    │    │                       (Datadog Platform)                        │
    //    └─────────────────────┘    └ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┘

    let dsd_config = DogStatsDConfiguration::from_configuration(config)
        .error_context("Failed to configure DogStatsD source.")?
        .with_workload_provider(env_provider.workload().clone())
        .with_capture_entity_resolver(env_provider.workload().clone());
    let dsd_prefix_filter_configuration = DogStatsDPrefixFilterConfiguration::from_configuration(config)?;
    let dsd_mapper_config = DogStatsDMapperConfiguration::from_configuration(config)?;
    let dsd_enrich_config =
        ChainedConfiguration::default().with_transform_builder("dogstatsd_mapper", dsd_mapper_config);
    let dsd_tag_filterlist_config = TagFilterlistConfiguration::from_configuration(config)
        .error_context("Failed to configure metric tag filterlist transform.")?;
    let dsd_agg_config =
        AggregateConfiguration::from_configuration(config).error_context("Failed to configure aggregate transform.")?;
    let dsd_post_agg_filter_config = DogStatsDPostAggregateFilterConfiguration::from_configuration(config)
        .error_context("Failed to configure DogStatsD post-aggregate filter transform.")?;
    let events_enrich_config = ChainedConfiguration::default().with_transform_builder(
        "host_enrichment",
        HostEnrichmentConfiguration::from_environment_provider(env_provider.clone()),
    );
    let service_checks_enrich_config = ChainedConfiguration::default().with_transform_builder(
        "host_enrichment",
        HostEnrichmentConfiguration::from_environment_provider(env_provider.clone()),
    );
    let dsd_debug_log_config = DogStatsDDebugLogConfiguration::from_configuration(
        config,
        PlatformSettings::get_default_dogstatsd_log_file_path(),
    )
    .error_context("Failed to configure DogStatsD debug log destination.")?;
    let dsd_stats_config = DogStatsDStatisticsConfiguration::new();

    let stats_api_handler = dsd_stats_config.api_handler();
    let capture_api_handler = dsd_config.capture_api_handler();
    let replay_api_handler = dsd_config.replay_api_handler();

    blueprint
        // Components.
        .add_source("dsd_in", dsd_config)?
        .add_transform("dsd_prefix_filter", dsd_prefix_filter_configuration)?
        .add_transform("dsd_enrich", dsd_enrich_config)?
        .add_transform("dsd_tag_filterlist", dsd_tag_filterlist_config)?
        .add_transform("dsd_agg", dsd_agg_config)?
        .add_transform("dsd_post_agg_filter", dsd_post_agg_filter_config)?
        .add_transform("events_enrich", events_enrich_config)?
        .add_transform("service_checks_enrich", service_checks_enrich_config)?
        .add_destination("dsd_stats_out", dsd_stats_config)?
        // Metrics.
        .connect_components_in_order([
            "dsd_in.metrics",
            "dsd_enrich",
            "dsd_prefix_filter",
            "dsd_tag_filterlist",
            "dsd_agg",
            "dsd_post_agg_filter",
            "metrics_enrich",
        ])?
        // Events.
        .connect_components_in_order(["dsd_in.events", "events_enrich", "dd_events_encode"])?
        // Service checks.
        .connect_components_in_order([
            "dsd_in.service_checks",
            "service_checks_enrich",
            "dd_service_checks_encode",
        ])?
        // DogStatsD Stats.
        .connect_components("dsd_in.metrics", "dsd_stats_out")?;

    if dsd_debug_log_config.enabled() {
        blueprint
            // DogStatsD debug log.
            .add_destination("dsd_debug_log_out", dsd_debug_log_config)?
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
    let dp_config = &saluki_config.data_plane;
    if dp_config.otlp().proxy().enabled() {
        let core_agent_otlp_grpc_endpoint = dp_config.otlp().proxy().core_agent_otlp_grpc_endpoint().to_string();
        let proxy_metrics = dp_config.otlp().proxy().proxy_metrics();
        let proxy_logs = dp_config.otlp().proxy().proxy_logs();
        let proxy_traces = dp_config.otlp().proxy().proxy_traces();

        info!(
            proxy_grpc_endpoint = %core_agent_otlp_grpc_endpoint,
            proxy_metrics,
            proxy_logs,
            proxy_traces,
            "OTLP proxy mode enabled. Select OTLP payloads will be proxied to the Core Agent."
        );

        let otlp_relay_config = OtlpRelayConfiguration::from_native(&saluki_config.otlp_receiver);
        let otlp_decoder_config = OtlpDecoderConfiguration::from_native(&saluki_config.otlp_traces);

        let local_agent_otlp_forwarder_config = OtlpForwarderConfiguration::from_native(&saluki_config.otlp_forwarder);

        blueprint
            // Components.
            .add_relay("otlp_relay_in", otlp_relay_config)?
            .add_forwarder("local_agent_otlp_out", local_agent_otlp_forwarder_config)?
            // Metrics and logs directly to the forwarders.
            .connect_components(["otlp_relay_in.metrics", "otlp_relay_in.logs"], "local_agent_otlp_out")?;

        if dp_config.otlp().proxy().proxy_traces() {
            blueprint.connect_components("otlp_relay_in.traces", "local_agent_otlp_out")?;
        } else {
            blueprint
                .add_decoder("otlp_traces_decode", otlp_decoder_config)?
                // Traces to decoder, then to the trace pipeline: obfuscation, enrichment, encoding, stats, forwarding.
                .connect_components_in_order(["otlp_relay_in.traces", "otlp_traces_decode", "traces_enrich"])?;
        }
    } else {
        info!("OTLP proxy mode disabled. OTLP signals will be handled natively.");

        let otlp_config = OtlpConfiguration::from_native(&saluki_config.otlp_source)
            .with_workload_provider(env_provider.workload().clone());

        blueprint
            // Components.
            .add_source("otlp_in", otlp_config)?
            // Metrics, logs, and traces.
            //
            // We send OTLP metrics directly to the enrichment stage of the metrics pipeline, skipping aggregation,
            // to avoid transforming counters into rates.
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
