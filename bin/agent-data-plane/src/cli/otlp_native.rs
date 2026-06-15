//! OTLP native-config assembly: the first runtime consumer of the typed translation boundary.
//!
//! This is the translation boundary for OTLP construction. It deserializes the typed
//! [`DatadogConfiguration`] from the resolved `GenericConfiguration`, applies the one OTLP env
//! override that the typed deserialize cannot capture on its own, runs the pure translator, and
//! bundles the resulting native OTLP slice together with the Saluki-private OTLP knobs (which are not
//! Datadog-schema keys and so have no place in `TotalSalukiConfiguration` yet).
//!
//! The bundle ([`OtlpNativeConfig`]) is the only OTLP input the topology builder needs. The OTLP
//! component constructors (`OtlpConfiguration::from_native`, relay/decoder/forwarder `from_native`)
//! consume it without ever touching `GenericConfiguration`.

use bytesize::ByteSize;
use datadog_agent_config::{translate, DatadogConfiguration};
use saluki_components::config::NativeTracesPrivateConfig;
use saluki_config::GenericConfiguration;
use saluki_error::{ErrorContext as _, GenericError};

/// Native OTLP boot-time configuration plus the Saluki-private knobs the components still need.
///
/// `otlp` is the translated native Datadog-schema OTLP slice. The remaining fields are
/// Saluki-private keys (not in the Datadog schema). They are read from `GenericConfiguration` here as
/// a temporary, explicit compatibility path until they move to `SalukiPrivateConfiguration` in a
/// later migration PR; see the field docs.
pub struct OtlpNativeConfig {
    /// Translated native OTLP config (Datadog-schema OTLP keys, sampling pct env override applied).
    pub otlp: datadog_agent_config::OtlpConfig,
    /// `otlp_string_interner_size`: context interner capacity for the native source.
    pub context_interner_bytes: ByteSize,
    /// `otlp_cached_contexts_limit`: native source resolved-context cache cap.
    pub cached_contexts_limit: usize,
    /// `otlp_cached_tagsets_limit`: native source resolved-tagset cache cap.
    pub cached_tagsets_limit: usize,
    /// `otlp_allow_context_heap_allocs`: allow heap allocation when the interner is full.
    pub allow_context_heap_allocs: bool,
    /// Saluki-private `otlp_config.traces.*` knobs consumed by the OTLP traces translator.
    pub traces_private: NativeTracesPrivateConfig,
}

// Defaults mirror the OTLP component serde defaults so the legacy and native paths agree when a knob
// is unset. They are duplicated here (rather than imported) because the component defaults are
// private to `saluki-components`; this duplication is temporary and disappears when these knobs move
// to `SalukiPrivateConfiguration`.
fn default_context_interner_bytes() -> ByteSize {
    ByteSize::mib(2)
}

const DEFAULT_CACHED_CONTEXTS_LIMIT: usize = 500_000;
const DEFAULT_CACHED_TAGSETS_LIMIT: usize = 500_000;
const DEFAULT_ALLOW_CONTEXT_HEAP_ALLOCS: bool = true;

fn default_traces_interner_bytes() -> ByteSize {
    ByteSize::kib(512)
}

const DEFAULT_IGNORE_MISSING_DATADOG_FIELDS: bool = false;
const DEFAULT_ENABLE_OTLP_COMPUTE_TOP_LEVEL_BY_SPAN_KIND: bool = true;

/// Flat Figment key that a `DD_*` OTLP sampling-percentage env var strips to.
///
/// Figment's `Env` provider splits keys on `__`, so the single-underscore env var
/// `DD_OTLP_CONFIG_TRACES_PROBABILISTIC_SAMPLER_SAMPLING_PERCENTAGE` becomes this one flat key rather
/// than the nested schema path. `DatadogConfiguration::deserialize` only reads the nested path, so it
/// cannot observe the env var on its own; see [`apply_sampling_percentage_env_override`].
const OTLP_SAMPLING_PERCENTAGE_FLAT_KEY: &str = "otlp_config_traces_probabilistic_sampler_sampling_percentage";

/// Builds the native OTLP boot config from the resolved `GenericConfiguration`.
///
/// Deserializes [`DatadogConfiguration`], applies the OTLP sampling-percentage env override that the
/// typed deserialize cannot capture, translates to native config, and reads the Saluki-private OTLP
/// knobs.
pub fn build_otlp_native_config(config: &GenericConfiguration) -> Result<OtlpNativeConfig, GenericError> {
    let mut dd_config: DatadogConfiguration = config
        .as_typed()
        .error_context("Failed to deserialize typed Datadog configuration for OTLP translation.")?;

    apply_sampling_percentage_env_override(&mut dd_config, config)?;

    let total_config = translate(&dd_config);

    let traces_private = NativeTracesPrivateConfig {
        string_interner_bytes: config
            .try_get_typed::<ByteSize>("otlp_config.traces.string_interner_size")?
            .unwrap_or_else(default_traces_interner_bytes),
        ignore_missing_datadog_fields: config
            .try_get_typed::<bool>("otlp_config.traces.ignore_missing_datadog_fields")?
            .unwrap_or(DEFAULT_IGNORE_MISSING_DATADOG_FIELDS),
        enable_otlp_compute_top_level_by_span_kind: config
            .try_get_typed::<bool>("otlp_config.traces.enable_otlp_compute_top_level_by_span_kind")?
            .unwrap_or(DEFAULT_ENABLE_OTLP_COMPUTE_TOP_LEVEL_BY_SPAN_KIND),
    };

    Ok(OtlpNativeConfig {
        otlp: total_config.otlp,
        context_interner_bytes: config
            .try_get_typed::<ByteSize>("otlp_string_interner_size")?
            .unwrap_or_else(default_context_interner_bytes),
        cached_contexts_limit: config
            .try_get_typed::<usize>("otlp_cached_contexts_limit")?
            .unwrap_or(DEFAULT_CACHED_CONTEXTS_LIMIT),
        cached_tagsets_limit: config
            .try_get_typed::<usize>("otlp_cached_tagsets_limit")?
            .unwrap_or(DEFAULT_CACHED_TAGSETS_LIMIT),
        allow_context_heap_allocs: config
            .try_get_typed::<bool>("otlp_allow_context_heap_allocs")?
            .unwrap_or(DEFAULT_ALLOW_CONTEXT_HEAP_ALLOCS),
        traces_private,
    })
}

/// Folds the flat OTLP sampling-percentage env value into the nested typed config before translation.
///
/// The Datadog Agent accepts `DD_OTLP_CONFIG_TRACES_PROBABILISTIC_SAMPLER_SAMPLING_PERCENTAGE`. That
/// env var strips to the flat Figment key [`OTLP_SAMPLING_PERCENTAGE_FLAT_KEY`], which
/// [`DatadogConfiguration::deserialize`] never sees because it reads only the nested schema path. We
/// therefore read the flat key explicitly (via `GenericConfiguration`'s `.`-to-`_` fallback lookup)
/// and patch it onto the typed config, so the translator carries it into
/// `total_config.otlp.traces_sampling_percentage`. This replaces the per-component
/// `TracesConfig::apply_env_overrides` step on the migrated native path; the override now happens
/// once, at the translation boundary.
fn apply_sampling_percentage_env_override(
    dd_config: &mut DatadogConfiguration, config: &GenericConfiguration,
) -> Result<(), GenericError> {
    if let Some(pct) = config.try_get_typed::<f64>(OTLP_SAMPLING_PERCENTAGE_FLAT_KEY)? {
        let otlp_config = dd_config.otlp_config.get_or_insert_with(Default::default);
        let traces = otlp_config.traces.get_or_insert_with(Default::default);
        let sampler = traces.probabilistic_sampler.get_or_insert_with(Default::default);
        sampler.sampling_percentage = pct;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use saluki_components::config::{DatadogRemapper, KEY_ALIASES};
    use saluki_config::ConfigurationLoader;
    use serde_json::json;

    use super::*;

    async fn config_with_env(file_values: serde_json::Value, env_vars: &[(String, String)]) -> GenericConfiguration {
        let (cfg, _) = ConfigurationLoader::for_tests_with_provider_factory(
            Some(file_values),
            Some(env_vars),
            false,
            KEY_ALIASES,
            DatadogRemapper::new,
        )
        .await;
        cfg
    }

    // The flat sampling-percentage env var must flow through translate() to the native value. This is
    // the override that DatadogConfiguration::deserialize cannot capture on its own (Figment splits
    // env keys on `__`, so the single-underscore var lands as a flat key, not the nested path).
    #[tokio::test]
    async fn sampling_percentage_env_var_reaches_native_config() {
        let env = [(
            "OTLP_CONFIG_TRACES_PROBABILISTIC_SAMPLER_SAMPLING_PERCENTAGE".to_string(),
            "12.5".to_string(),
        )];
        let cfg = config_with_env(json!({ "otlp_config": {} }), &env).await;

        let native = build_otlp_native_config(&cfg).expect("should build native OTLP config");
        assert_eq!(native.otlp.traces_sampling_percentage, 12.5);
    }

    // The same value set via the nested YAML path must produce the same native value, confirming the
    // boundary treats file and env sources identically.
    #[tokio::test]
    async fn sampling_percentage_yaml_reaches_native_config() {
        let cfg = config_with_env(
            json!({
                "otlp_config": { "traces": { "probabilistic_sampler": { "sampling_percentage": 12.5 } } }
            }),
            &[],
        )
        .await;

        let native = build_otlp_native_config(&cfg).expect("should build native OTLP config");
        assert_eq!(native.otlp.traces_sampling_percentage, 12.5);
    }

    // With nothing set, the native value is the schema default and the Saluki-private knobs fall back
    // to their component defaults.
    #[tokio::test]
    async fn defaults_when_unset() {
        let cfg = config_with_env(json!({ "otlp_config": {} }), &[]).await;

        let native = build_otlp_native_config(&cfg).expect("should build native OTLP config");
        assert_eq!(native.otlp.traces_sampling_percentage, 100.0);
        assert_eq!(native.context_interner_bytes, ByteSize::mib(2));
        assert_eq!(native.cached_contexts_limit, 500_000);
        assert_eq!(native.cached_tagsets_limit, 500_000);
        assert!(native.allow_context_heap_allocs);
        assert_eq!(native.traces_private.string_interner_bytes, ByteSize::kib(512));
        assert!(!native.traces_private.ignore_missing_datadog_fields);
        assert!(native.traces_private.enable_otlp_compute_top_level_by_span_kind);
    }

    // The Saluki-private knobs are read from GenericConfiguration and reach the bundle.
    #[tokio::test]
    async fn saluki_private_knobs_are_read() {
        let cfg = config_with_env(
            json!({
                "otlp_string_interner_size": "4Mib",
                "otlp_cached_contexts_limit": 123,
                "otlp_cached_tagsets_limit": 456,
                "otlp_allow_context_heap_allocs": false,
                "otlp_config": {
                    "traces": {
                        "string_interner_size": "1Mib",
                        "ignore_missing_datadog_fields": true,
                        "enable_otlp_compute_top_level_by_span_kind": false
                    }
                }
            }),
            &[],
        )
        .await;

        let native = build_otlp_native_config(&cfg).expect("should build native OTLP config");
        assert_eq!(native.context_interner_bytes, ByteSize::mib(4));
        assert_eq!(native.cached_contexts_limit, 123);
        assert_eq!(native.cached_tagsets_limit, 456);
        assert!(!native.allow_context_heap_allocs);
        assert_eq!(native.traces_private.string_interner_bytes, ByteSize::mib(1));
        assert!(native.traces_private.ignore_missing_datadog_fields);
        assert!(!native.traces_private.enable_otlp_compute_top_level_by_span_kind);
    }
}
