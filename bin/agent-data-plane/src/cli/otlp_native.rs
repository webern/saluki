//! OTLP native-config assembly over the typed translation boundary.
//!
//! Translation itself does not live here. The resolved `GenericConfiguration` is deserialized into
//! the typed [`DatadogConfiguration`], the few env overrides the typed deserialize cannot capture are
//! folded onto it, and the pure translator is run -- all once, unconditionally, in the run path (see
//! [`build_total_config`]). This module then bundles the translated native OTLP slice together with
//! the Saluki-private OTLP knobs (which are not Datadog-schema keys and so have no place in
//! `TotalSalukiConfiguration` yet) via [`build_otlp_native_config`].
//!
//! The bundle ([`OtlpNativeConfig`]) is the only OTLP input the topology builder needs. The OTLP
//! component constructors (`OtlpConfiguration::from_native`, relay/decoder/forwarder `from_native`)
//! consume it without ever touching `GenericConfiguration`.

use bytesize::ByteSize;
use datadog_agent_config::{translate, DatadogConfiguration, TotalSalukiConfiguration};
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

/// Builds the complete native [`TotalSalukiConfiguration`] from the resolved `GenericConfiguration`.
///
/// This is the single translation boundary for the run path: it deserializes the typed
/// [`DatadogConfiguration`], folds the OTLP boundary overrides that the typed deserialize cannot
/// capture on its own (the sampling-percentage env var and the legacy logs-enabled default), and runs
/// the pure translator -- exactly once, unconditionally, regardless of which pipelines are enabled.
/// Each migrated subsystem (OTLP today; traces/logs/metrics/... in later PRs) then consumes its slice
/// of the returned `TotalSalukiConfiguration`.
pub fn build_total_config(config: &GenericConfiguration) -> Result<TotalSalukiConfiguration, GenericError> {
    let mut dd_config: DatadogConfiguration = config
        .as_typed()
        .error_context("Failed to deserialize typed Datadog configuration for translation.")?;

    apply_sampling_percentage_env_override(&mut dd_config, config)?;
    apply_legacy_logs_enabled_default(&mut dd_config, config)?;

    Ok(translate(&dd_config))
}

/// Builds the native OTLP boot config from the translated config plus the Saluki-private OTLP knobs.
///
/// The OTLP slice comes from the already-translated `total_config`; this function does NOT translate.
/// The Saluki-private knobs (the four interner/cache knobs and the `otlp_config.traces.*` private
/// knobs) are not Datadog-schema keys, so they have no place in `TotalSalukiConfiguration` and are
/// read here from `GenericConfiguration` as a temporary compatibility path until they move to
/// `SalukiPrivateConfiguration` in a later migration PR.
pub fn build_otlp_native_config(
    total_config: &TotalSalukiConfiguration, config: &GenericConfiguration,
) -> Result<OtlpNativeConfig, GenericError> {
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
        otlp: total_config.otlp.clone(),
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

/// Folds the legacy ADP "OTLP logs on by default" behavior onto the typed config before translation.
///
/// Legacy ADP defaulted `otlp_config.logs.enabled` to `true` (the `default_logs_enabled` serde
/// default in `saluki-components`), whereas the vendored Datadog Agent schema defaults it to `false`.
/// This is a behavior-preserving migration PR, so we reproduce the legacy default here: if the
/// operator did not set the key at all, we fold `true` onto the typed config so the native
/// `logs_enabled` resolves to true; if the operator set it explicitly, we honor their value.
///
/// We read the RAW key from `GenericConfiguration` to distinguish "absent" from "explicit default":
/// the typed `DatadogConfiguration` already collapses both onto `false` (its schema default), so it
/// cannot tell them apart on its own.
///
/// Adopting the Agent schema default (logs off when unset) is a deliberate future decision, NOT this
/// PR's: changing it is a behavior change that belongs in its own change with its own justification.
fn apply_legacy_logs_enabled_default(
    dd_config: &mut DatadogConfiguration, config: &GenericConfiguration,
) -> Result<(), GenericError> {
    match config.try_get_typed::<bool>("otlp_config.logs.enabled")? {
        // Operator set it explicitly: honor their value (the typed deserialize already captured it).
        Some(_) => {}
        // Operator did not set it: preserve the legacy ADP logs-on-by-default behavior.
        None => {
            let otlp_config = dd_config.otlp_config.get_or_insert_with(Default::default);
            let logs = otlp_config.logs.get_or_insert_with(Default::default);
            logs.enabled = true;
        }
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

    /// Translate then bundle, mirroring the run-path order (single `translate()` then bundle assembly).
    fn build_native(cfg: &GenericConfiguration) -> OtlpNativeConfig {
        let total_config = build_total_config(cfg).expect("should build total config");
        build_otlp_native_config(&total_config, cfg).expect("should build native OTLP config")
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

        let native = build_native(&cfg);
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

        let native = build_native(&cfg);
        assert_eq!(native.otlp.traces_sampling_percentage, 12.5);
    }

    // With nothing set, the native value is the schema default and the Saluki-private knobs fall back
    // to their component defaults. The grpc/http endpoint defaults are asserted here so the boot-bundle
    // test is self-contained.
    #[tokio::test]
    async fn defaults_when_unset() {
        let cfg = config_with_env(json!({ "otlp_config": {} }), &[]).await;

        let native = build_native(&cfg);
        assert_eq!(native.otlp.traces_sampling_percentage, 100.0);
        assert_eq!(native.otlp.grpc.endpoint, "localhost:4317");
        assert_eq!(native.otlp.http.endpoint, "localhost:4318");
        assert_eq!(native.context_interner_bytes, ByteSize::mib(2));
        assert_eq!(native.cached_contexts_limit, 500_000);
        assert_eq!(native.cached_tagsets_limit, 500_000);
        assert!(native.allow_context_heap_allocs);
        assert_eq!(native.traces_private.string_interner_bytes, ByteSize::kib(512));
        assert!(!native.traces_private.ignore_missing_datadog_fields);
        assert!(native.traces_private.enable_otlp_compute_top_level_by_span_kind);
    }

    // Legacy behavior preservation: ADP defaulted OTLP logs ON. When the operator does not set
    // `otlp_config.logs.enabled`, the boundary folds `true` onto the typed config so the native
    // `logs_enabled` stays on, matching pre-migration ADP. Adopting the Agent schema default (off) is a
    // deliberate future decision, not this PR's.
    #[tokio::test]
    async fn logs_enabled_absent_preserves_legacy_on() {
        let cfg = config_with_env(json!({ "otlp_config": {} }), &[]).await;

        let total_config = build_total_config(&cfg).expect("should build total config");
        assert!(
            total_config.otlp.logs_enabled,
            "absent otlp_config.logs.enabled must preserve legacy ADP logs-on default"
        );
    }

    // An explicit `false` is honored: the operator opted out of OTLP logs.
    #[tokio::test]
    async fn logs_enabled_explicit_false_is_off() {
        let cfg = config_with_env(json!({ "otlp_config": { "logs": { "enabled": false } } }), &[]).await;

        let total_config = build_total_config(&cfg).expect("should build total config");
        assert!(!total_config.otlp.logs_enabled, "explicit false must disable OTLP logs");
    }

    // An explicit `true` is honored.
    #[tokio::test]
    async fn logs_enabled_explicit_true_is_on() {
        let cfg = config_with_env(json!({ "otlp_config": { "logs": { "enabled": true } } }), &[]).await;

        let total_config = build_total_config(&cfg).expect("should build total config");
        assert!(total_config.otlp.logs_enabled, "explicit true must enable OTLP logs");
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

        let native = build_native(&cfg);
        assert_eq!(native.context_interner_bytes, ByteSize::mib(4));
        assert_eq!(native.cached_contexts_limit, 123);
        assert_eq!(native.cached_tagsets_limit, 456);
        assert!(!native.allow_context_heap_allocs);
        assert_eq!(native.traces_private.string_interner_bytes, ByteSize::mib(1));
        assert!(native.traces_private.ignore_missing_datadog_fields);
        assert!(!native.traces_private.enable_otlp_compute_top_level_by_span_kind);
    }
}
