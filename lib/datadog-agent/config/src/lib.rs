pub mod classifier;

/// Generated typed deserializer for the supported Datadog Agent configuration surface.
///
/// Generated at build time from `core_schema.yaml` plus `schema_overlay.yaml`. Contains only keys
/// inventoried as `support: full` or `support: partial`. Mostly unused until the configuration
/// translator consumes it.
pub mod datadog_configuration;

/// Generated witness trait (`DatadogConfigConsumer`) and driver (`drive`) over the supported keys.
///
/// Generated at build time from `schema_overlay.yaml`, sharing its leaf types with
/// `datadog_configuration`. The witness makes supported-key coverage a compile-time guarantee for
/// the hand-written translator.
pub mod witness;

/// Native Saluki configuration structs assembled by the translator.
pub mod total_config;

/// Hand-written translation from `DatadogConfiguration` to `TotalSalukiConfiguration`.
pub mod translator;

pub use datadog_configuration::DatadogConfiguration;
pub use total_config::TotalSalukiConfiguration;
pub use translator::translate;
pub use witness::{drive, DatadogConfigConsumer};
