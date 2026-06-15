//! ADP-native configuration model.
//!
//! This crate owns the target side of configuration translation: the typed configuration ADP runs
//! with after source-specific loading is complete. It contains lifecycle and runtime concepts such
//! as [`BootstrapConfiguration`], [`SalukiPrivateConfiguration`], [`RuntimeConfigAuthority`], and
//! [`SalukiConfiguration`].
//!
//! # Forbidden dependencies
//!
//! This crate must not depend on `datadog-agent-config` or `saluki_config::GenericConfiguration`,
//! and it must not contain Datadog key names, schema defaults, environment aliases, or raw-map
//! loading. Keeping this model source-agnostic lets Datadog, Saluki-private, OTel, or future inputs
//! all translate into the same ADP-owned runtime shape. It *may* depend down on
//! `saluki-component-config` so [`SalukiConfiguration`] embeds the real component config structs
//! directly, with no duplicate-and-convert hop at topology build time.

#![deny(missing_docs)]
#![deny(warnings)]

pub mod authority;
pub mod bootstrap;
pub mod logging;
pub mod private;
pub mod saluki;

pub use authority::{ConfigStreamAuthority, RuntimeConfigAuthority, RuntimeConfigLanguage};
pub use bootstrap::{
    BootstrapConfiguration, BootstrapIpcConfiguration, BootstrapStartupConfiguration, BootstrapTelemetryConfiguration,
};
pub use logging::RuntimeLoggingConfig;
pub use private::SalukiPrivateConfiguration;
pub use saluki::{
    ChecksConfigs, DataPlaneConfig, DogStatsDConfigs, EventsConfigs, ForwarderConfigs, LogsConfigs, MetricsConfigs,
    OtlpConfigs, PipelineGate, SalukiConfiguration, ServiceChecksConfigs, TracesConfigs,
};
