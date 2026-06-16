//! ADP-native configuration model.
//!
//! This crate owns the target side of configuration translation: the typed configuration ADP wants
//! to run with after source-specific loading is complete. It contains lifecycle and runtime concepts
//! such as `BootstrapConfiguration`, `SalukiPrivateConfiguration`, `RuntimeConfigAuthority`, and
//! `SalukiConfiguration`.
//!
//! This crate must not depend on `datadog-agent-config` or `saluki-config::GenericConfiguration`.
//! Datadog key names, schema defaults, environment aliases, and raw-map loading belong outside this
//! model. Keeping this crate source-agnostic lets Datadog, Saluki-private, OTel, or future inputs
//! translate into the same ADP-owned runtime shape.

pub mod authority;
pub mod bootstrap;
pub mod private;
pub mod saluki;

pub use authority::{ConfigStreamAuthority, RuntimeConfigAuthority, RuntimeConfigLanguage};
pub use bootstrap::{BootstrapConfiguration, BootstrapStartupConfiguration, BootstrapTelemetryConfiguration};
pub use private::SalukiPrivateConfiguration;
pub use saluki::{DataPlaneConfiguration, EnvironmentConfiguration, SalukiConfiguration};
pub use saluki_component_config::{
    ChecksIpcConfiguration, DatadogEventsEncoderConfiguration, DatadogLogsEncoderConfiguration,
    DatadogServiceChecksEncoderConfiguration, OtlpForwarderConfiguration, OtlpPipelineConfiguration,
    OtlpProxyConfiguration, PipelineConfiguration,
};

#[cfg(test)]
mod tests {
    #[test]
    fn cargo_toml_does_not_take_source_model_dependencies() {
        let manifest = include_str!("../Cargo.toml");

        assert!(!manifest.contains("datadog-agent-config"));
        assert!(!manifest.contains("saluki-config"));
    }
}
