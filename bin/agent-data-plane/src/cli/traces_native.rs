//! Traces/APM native-config assembly over the typed translation boundary.
//!
//! The `TracesNativeConfig` struct lives in `saluki-components` (alongside `ApmConfig`) so the five
//! trace-pipeline components can import it without a dependency on this binary. This module contains
//! only the builder function `build_traces_native_config`, which reads the already-translated
//! `total_config.otlp.traces_sampling_percentage` and calls `ApmConfig::from_configuration` once for
//! all five components.
//!
//! ApmConfig sampling fields (`target_traces_per_second`, `errors_per_second`, rare sampler, etc.) are
//! still read via `ApmConfig::from_configuration` because those keys are "ignored" in the schema overlay
//! and are absent from `DatadogConfiguration`. Only `otlp.traces_sampling_percentage` comes from the
//! typed translation path. Promoting sampling keys to the overlay is deferred.

use datadog_agent_config::TotalSalukiConfiguration;
use saluki_components::config::{ApmConfig, TracesNativeConfig};
use saluki_config::GenericConfiguration;
use saluki_error::GenericError;

/// Builds the native traces boot config from the translated config and the raw `GenericConfiguration`.
///
/// Calls `ApmConfig::from_configuration` exactly once, folds the translated OTLP sampling percentage
/// from `total_config` into the bundle, and returns the result for all five trace-pipeline
/// components to consume via their `from_native` constructors.
pub fn build_traces_native_config(
    total_config: &TotalSalukiConfiguration, config: &GenericConfiguration,
) -> Result<TracesNativeConfig, GenericError> {
    let apm_config = ApmConfig::from_configuration(config)?;
    Ok(TracesNativeConfig {
        apm_config,
        otlp_traces_sampling_percentage: total_config.otlp.traces_sampling_percentage,
    })
}
