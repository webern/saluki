//! Bootstrap configuration model.

use crate::authority::RuntimeConfigAuthority;

/// Typed configuration needed before runtime configuration authority is available.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapConfiguration {
    /// Startup decisions derived from local bootstrap sources.
    pub startup: BootstrapStartupConfiguration,

    /// Early process telemetry configuration.
    pub telemetry: BootstrapTelemetryConfiguration,
}

/// Startup decisions that determine how runtime configuration is resolved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapStartupConfiguration {
    /// The selected runtime configuration authority.
    pub runtime_config_authority: RuntimeConfigAuthority,
}

/// Early telemetry settings needed before runtime configuration is online.
///
/// TODO: Decide whether these can be removed from bootstrap by using fixed defaults until runtime
/// configuration is translated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapTelemetryConfiguration {
    /// TODO: figure out the actual struct fields needed.
    pub metrics_level: Option<String>,
}
