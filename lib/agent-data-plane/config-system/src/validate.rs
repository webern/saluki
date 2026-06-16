//! Overlay-classifier validation of the resolved Datadog configuration.
//!
//! The design places overlay/classifier validation inside the configuration system, before
//! `SalukiConfiguration` is returned — `run.rs` should never flatten or inspect source-language
//! keys. This runs the classifier over the flattened raw configuration and rejects high-severity
//! incompatibilities for active pipelines.

use std::collections::HashSet;

use datadog_agent_config::classifier::{ConfigClassifier, Pipeline, PipelineAffinity, Severity, SupportLevel};
use saluki_config::GenericConfiguration;
use saluki_error::{generic_error, ErrorContext as _, GenericError};
use tracing::{debug, error, trace, warn};

use crate::translate::PipelineGates;

/// Classify each flattened key in `config`, warning on partial/medium incompatibilities and erroring
/// on any high-severity incompatibility whose affected pipeline is active.
pub(crate) fn validate_against_overlay(
    config: &GenericConfiguration, gates: PipelineGates,
) -> Result<(), GenericError> {
    let active = active_pipelines(gates);
    let classifier = ConfigClassifier::new();
    let mut high_severity = 0u32;

    debug!("Analyzing configuration against the overlay classifier.");
    for (key, val) in config
        .flattened_keys()
        .error_context("Unable to flatten configuration into a list of dot-separated keys.")?
    {
        let Some(classification) = classifier.classify(&key, &val) else {
            continue;
        };

        if !is_a_pipeline_affected(&active, &classification.pipeline_affinity) {
            continue;
        }

        if classification.is_default {
            trace!(key = %key, "Configuration key has a default value.");
            continue;
        }

        match classification.support_level {
            SupportLevel::Incompatible(Severity::Low) => {
                debug!(key = %key, "Low-severity incompatible key detected. Proceeding.")
            }
            SupportLevel::Partial => {
                warn!(key = %key, "Partially supported configuration key. See documentation for details. Proceeding.")
            }
            SupportLevel::Incompatible(Severity::Medium) => {
                warn!(key = %key, "Unsupported configuration key. Proceeding.")
            }
            SupportLevel::Incompatible(Severity::High) => {
                error!(key = %key, "Unsupported configuration key with non-default value. ADP cannot run safely with this setting.");
                high_severity += 1;
            }
            SupportLevel::Ignored | SupportLevel::Unrecognized => {
                trace!(key = %key, "Configuration key not-applicable. Silently ignoring.")
            }
        }
    }

    if high_severity > 0 {
        return Err(generic_error!(
            "{high_severity} incompatible configuration detected. ADP cannot start. Review error logs for details."
        ));
    }

    Ok(())
}

fn active_pipelines(gates: PipelineGates) -> HashSet<Pipeline> {
    let mut active = HashSet::new();
    if gates.dogstatsd_enabled {
        active.insert(Pipeline::DogStatsD);
    }
    if gates.checks_enabled {
        active.insert(Pipeline::Checks);
    }
    if gates.otlp_enabled {
        active.insert(Pipeline::Otlp);
        // The traces pipeline is required for native (non-proxy) OTLP; conservatively mark it active
        // whenever OTLP is enabled so trace-affecting keys are classified.
        active.insert(Pipeline::Traces);
    }
    active
}

fn is_a_pipeline_affected(active: &HashSet<Pipeline>, affinity: &PipelineAffinity) -> bool {
    match affinity {
        PipelineAffinity::Pipelines(affected) => affected.iter().any(|p| active.contains(p)),
        PipelineAffinity::CrossCutting => true,
    }
}
