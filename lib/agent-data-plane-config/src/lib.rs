//! ADP-native configuration model: the typed target of configuration translation.
//!
//! This crate owns the domain-shaped model types that translation produces and that ADP runtime
//! code consumes: `SalukiConfiguration { control, shared, domains }`, `ControlConfiguration`,
//! `SharedConfiguration`, `DomainConfiguration` and its per-domain structs.
//!
//! It does not embed component config structs (those stay in `saluki-components`, built from this
//! model). It depends on neither the raw configuration map nor the Datadog source model, so a
//! consumer can depend on it without inheriting either.
//!
//! Every field is plain, source-agnostic data. There are no source key names in identifiers and no
//! source serde (these structs are serialized for the `/config/internal` view but never
//! deserialized from a source language; that is the source adapter's job).

use serde::Serialize;

pub mod control;
pub mod domains;
pub mod shared;

pub use control::{ControlConfiguration, ListenAddress, Logging};
pub use domains::DomainConfiguration;
pub use shared::SharedConfiguration;

/// The complete ADP-native runtime configuration after translation.
///
/// Two writers fill it: the Datadog witness `drive` (schema fields) and `seed` (Saluki-only
/// fields). They write disjoint fields. It is read by the orchestration layer (`control`) and by
/// components at topology assembly (`shared` and `domains`).
#[derive(Clone, Debug, Default, Serialize)]
pub struct SalukiConfiguration {
    /// Read first: decides which pipelines/topology to build. Orchestration layer only.
    pub control: ControlConfiguration,
    /// Cross-cutting values consumed by more than one domain, each with a single home.
    pub shared: SharedConfiguration,
    /// Per-domain resolved config, grouped by ownership domain.
    pub domains: DomainConfiguration,
}

// COVERAGE
//
// Classification of all 237 inventoried source keys against the vendored schema. 224 map to a model
// field (154 witnessed directly, 24 promoted to witnessed, 46 seeded as Saluki-only); 13 have no
// field (4 deferred, 4 excluded, 5 subsumed).
//
// Notable modeling decisions:
//   - metrics_level lives in shared.metrics_level: it is read across the runtime, not by a single
//     component.
//   - ottl_filter_config / ottl_transform_config are reachable in the traces enrichment chain
//     (gated on the traces pipeline) and are seeded into domains.traces.ottl_filter / .ottl_transform.
//   - dogstatsd_expiry_seconds and its Saluki alias counter_expiry_seconds collapse to one field,
//     domains.dogstatsd.aggregation.counter_expiry_seconds.
//   - statsd_metric_namespace_blacklist and its modern alias statsd_metric_namespace_blocklist
//     collapse to one field, domains.dogstatsd.prefix_filter.metric_namespace_blocklist (the
//     generated source model folds one into the other via a serde alias).
//   - expected_tags_duration is typed as a number but its schema default is a Go-duration string
//     ("0s"); the source generator rewrites that default to 0.0.
//
// Seeded Saluki-only fields (46) group by destination as: control (6), shared (3),
// domains.dogstatsd (16), domains.otlp (5), domains.traces (15), domains.checks (1). Seven of the
// seeded fields use nested apm_config.* / data_plane.* paths that ADP reads from config but that are
// absent from the vendored schema (default_env, error_sampling_enabled, the three rare_sampler
// knobs, checks, standalone_mode), so they are seeded rather than driven.
//
// No-field keys (13):
//   - Deferred (4): owned by saluki-env and not cut over here (hostname, container_proc_root,
//     container_cgroup_root, cri_socket_path).
//   - Excluded (4): handled by the source adapter or dead for ADP (run_path, secret_backend_command,
//     secret_refresh_on_api_key_failure_interval, prometheus_listen_addr).
//   - Subsumed (5): enable_payloads (container for its four leaves), otlp_config.traces (subtree),
//     url (EndpointConfig.url), counter_expiry_seconds and statsd_metric_namespace_blocklist (aliases
//     folded above).
//
// shared.endpoints.endpoints is the only field assembled from several keys: the translator's
// finish() builds it from api_key, site, dd_url, and additional_endpoints. The alternate metrics
// intakes (observability_pipelines_worker.* and vector.*) are modeled under shared.endpoints as
// AltMetricsIntake.
