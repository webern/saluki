//! Datadog Agent configuration source model.
//!
//! This crate owns the Datadog side of ADP configuration: the generated `DatadogConfiguration`,
//! overlay-backed classification metadata, and source-side witness driver. It mirrors the supported
//! Datadog Agent schema surface; it should not own ADP-native runtime configuration.
//!
//! The ADP target model belongs in `agent-data-plane-config`. The adapter that translates from this
//! crate into that target model belongs in `agent-data-plane-config-system`. Keeping those crates
//! separate prevents Datadog schema concepts from becoming component APIs.

pub mod classifier;

/// Generated typed deserializer for the supported Datadog Agent configuration surface.
///
/// Generated at build time from `core_schema.yaml` plus `schema_overlay.yaml`. Contains only keys
/// inventoried as `support: full` or `support: partial`. Mostly unused until the configuration
/// translator consumes it.
pub mod datadog_configuration;

pub use datadog_configuration::DatadogConfiguration;
