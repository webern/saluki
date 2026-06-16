//! Realized shape of the collapsed `run.rs` startup seam (build-order step 10).
//!
//! This example compiles against the real configuration-system API to show what `run.rs` becomes
//! once the configuration system owns config resolution: one `ConfigurationSystem::start()` call
//! returns typed outputs, and the binary consumes `SalukiConfiguration` slices plus typed provider
//! attachments — never `GenericConfiguration`, never a `from_configuration` component call, never
//! local-vs-stream authority switching.
//!
//! It is illustrative (the topology/internal-supervisor calls are sketched in comments where they
//! would consume native slices); it is not run as part of any test. Building it proves the typed
//! boundary is directly consumable.

use agent_data_plane_config_system::{BootstrapInputs, ConfigurationSystem, StartedAttachments};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), saluki_error::GenericError> {
    // `main.rs` builds `BootstrapInputs` instead of a `GenericConfiguration`; the config system
    // consumes all raw local sources internally.
    let bootstrap_inputs = BootstrapInputs::default();

    // The binary supplies the gRPC service names it will expose; the config system only advertises
    // them during remote-agent registration.
    let service_names = vec![
        "datadog.remoteagent.status.v1.StatusProvider".to_string(),
        "datadog.remoteagent.flare.v1.FlareProvider".to_string(),
        "datadog.remoteagent.telemetry.v1.TelemetryProvider".to_string(),
    ];

    // The single startup seam: load bootstrap sources, parse `BootstrapConfiguration`, choose the
    // runtime authority, connect/register/stream only when required, load Saluki-private config, and
    // translate into `SalukiConfiguration`.
    let started = ConfigurationSystem::new(bootstrap_inputs, service_names)
        .start()
        .await?;

    let saluki_config = started.saluki();

    // Runtime logging reload would map `saluki_config.logging` (native `RuntimeLoggingConfig`) into
    // the application's logging stack here.
    let _log_level = saluki_config.logging.log_level.clone();

    if !saluki_config.data_plane.enabled() {
        // ADP is disabled; nothing to run.
        return Ok(());
    }

    // Topology assembly consumes native slices directly — no `from_configuration`, no raw map.
    // Each `add_*` below would build its component from the embedded native config struct.
    if saluki_config.data_plane.requires_datadog_forwarder() {
        let _forwarder_config = &saluki_config.forwarder.datadog; // -> blueprint.add_forwarder("dd_out", ...)
    }
    if saluki_config.data_plane.metrics_pipeline_required() {
        let _enrichment = &saluki_config.metrics.enrichment; // -> blueprint.add_transform("metrics_enrich", ...)
        let _encoder = &saluki_config.metrics.datadog_encoder; // -> blueprint.add_encoder("dd_metrics_encode", ...)
    }
    if saluki_config.data_plane.dogstatsd.enabled() {
        let _dsd = &saluki_config.dogstatsd.source; // -> blueprint.add_source("dsd_in", ...)
    }
    if saluki_config.data_plane.otlp.enabled() {
        let _otlp = &saluki_config.otlp.config; // -> add_otlp_pipeline_to_blueprint(...)
    }

    // Typed dynamic updates: when a stream-backed authority is active, the configuration system owns
    // the stream and the update router (spawned inside `start()`), and routes typed, scoped deltas to
    // per-component handles. The binary receives only the typed `DynamicConfigHandles`. Each component
    // holds only its own handle (for example the forwarder's), so a change elsewhere physically cannot
    // reach it.
    match started.attachments() {
        StartedAttachments::None => {
            // Local snapshot authority: no live updates.
        }
        StartedAttachments::DatadogAgentConfigStream { handles, .. } => {
            // `handles.forwarder`, `handles.prefix_filter`, `handles.tag_filterlist`,
            // `handles.dogstatsd_source`, and `handles.log_level` would be handed to their components
            // (for example via `with_dynamic_handle`) so each observes only its own slice's changes.
            let _ = &handles.forwarder;
        }
    }

    Ok(())
}
